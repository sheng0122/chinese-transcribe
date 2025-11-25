use actix_multipart::Multipart;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use futures::{StreamExt, TryStreamExt};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use async_channel;
use uuid::Uuid;

use transcribe_rs::worker::{Task, TaskMap, TaskStatus, Worker};

const DEFAULT_NUM_WORKERS: usize = 3;

struct AppState {
    tasks: TaskMap,
    sender: async_channel::Sender<String>,
}

async fn index() -> HttpResponse {
    let html = r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>Transcribe-rs Upload</title>
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <style>
            body { font-family: sans-serif; max-width: 600px; margin: 0 auto; padding: 20px; }
            .container { border: 1px solid #ccc; padding: 20px; border-radius: 8px; }
            input, button { margin: 10px 0; width: 100%; padding: 10px; box-sizing: border-box; }
            button { background: #007bff; color: white; border: none; cursor: pointer; }
            button:hover { background: #0056b3; }
            #status { margin-top: 20px; white-space: pre-wrap; }
        </style>
    </head>
    <body>
        <div class="container">
            <h2>Upload Audio for Transcription</h2>
            <input type="file" id="fileInput" accept=".wav,.mp3,.m4a">
            <button onclick="uploadFile()">Upload</button>
            <div id="status"></div>
        </div>

        <script>
            async function uploadFile() {
                const fileInput = document.getElementById('fileInput');
                const statusDiv = document.getElementById('status');
                
                if (!fileInput.files[0]) {
                    alert('Please select a file');
                    return;
                }

                const file = fileInput.files[0];
                const formData = new FormData();
                formData.append('file', file);

                statusDiv.textContent = 'Uploading...';

                try {
                    const response = await fetch('/upload', {
                        method: 'POST',
                        body: formData
                    });
                    
                    if (!response.ok) throw new Error('Upload failed');
                    
                    const data = await response.json();
                    const taskId = data.task_id;
                    
                    statusDiv.textContent = `Upload successful! Task ID: ${taskId}\nWaiting for transcription...`;
                    
                    pollStatus(taskId);
                } catch (e) {
                    statusDiv.textContent = 'Error: ' + e.message;
                }
            }

            async function pollStatus(taskId) {
                const statusDiv = document.getElementById('status');
                
                const interval = setInterval(async () => {
                    try {
                        const res = await fetch(`/status/${taskId}`);
                        const data = await res.json();
                        
                        if (data.status === 'Completed') {
                            clearInterval(interval);
                            statusDiv.innerHTML = `Transcription Completed!<br><a href="/download/${taskId}" target="_blank">Download SRT</a>`;
                        } else if (data.status.startsWith('Failed')) {
                            clearInterval(interval);
                            statusDiv.textContent = 'Failed: ' + data.status;
                        } else {
                            statusDiv.textContent = `Status: ${data.status}...`;
                        }
                    } catch (e) {
                        console.error(e);
                    }
                }, 2000);
            }
        </script>
    </body>
    </html>
    "#;

    HttpResponse::Ok().content_type("text/html").body(html)
}

async fn upload(
    mut payload: Multipart,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let mut task_id = String::new();

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let filename = content_disposition
            .get_filename()
            .map_or_else(|| Uuid::new_v4().to_string(), |f| f.to_string());

        let id = Uuid::new_v4().to_string();
        task_id = id.clone();
        
        let filepath = PathBuf::from(format!("uploads/{}", id)); // Use ID as filename to avoid collisions
        // Ensure uploads dir exists
        fs::create_dir_all("uploads")?;

        // Blocking file write to avoid blocking async runtime with heavy IO? 
        // For small chunks it's fine, but for large files `web::block` is better.
        // Here we use simple async stream writing.
        let filepath_clone = filepath.clone();
        let mut f = web::block(move || std::fs::File::create(filepath_clone))
            .await??;

        while let Some(chunk) = field.next().await {
            let data = chunk?;
            f = web::block(move || f.write_all(&data).map(|_| f))
                .await??;
        }

        // Create Task
        let task = Task {
            id: id.clone(),
            status: TaskStatus::Queued,
            original_filename: filename,
            file_path: filepath,
            result_srt: None,
        };

        // Add to map
        data.tasks.lock().unwrap().insert(id.clone(), task);

        // Send to worker
        data.sender.send(id).await.expect("Worker channel closed");
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({ "task_id": task_id })))
}

async fn get_status(path: web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    let task_id = path.into_inner();
    let tasks = data.tasks.lock().unwrap();

    if let Some(task) = tasks.get(&task_id) {
        HttpResponse::Ok().json(task)
    } else {
        HttpResponse::NotFound().body("Task not found")
    }
}

async fn download_srt(path: web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    let task_id = path.into_inner();
    let tasks = data.tasks.lock().unwrap();

    if let Some(task) = tasks.get(&task_id) {
        if let Some(srt) = &task.result_srt {
            HttpResponse::Ok()
                .content_type("text/plain")
                .insert_header(("Content-Disposition", format!("attachment; filename=\"{}.srt\"", task.id)))
                .body(srt.clone())
        } else {
            HttpResponse::BadRequest().body("Result not ready")
        }
    } else {
        HttpResponse::NotFound().body("Task not found")
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    let model_path = std::env::args().nth(1).unwrap_or_else(|| "models/whisper-medium-q4_1.bin".to_string());
    let num_workers: usize = std::env::var("NUM_WORKERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_NUM_WORKERS);

    println!("Server starting...");
    println!("Using model: {}", model_path);
    println!("Number of workers: {}", num_workers);

    // Shared state
    let tasks: TaskMap = Arc::new(Mutex::new(HashMap::new()));
    let (tx, rx) = async_channel::bounded(100);

    // Spawn multiple workers
    for worker_id in 0..num_workers {
        let worker_tasks = tasks.clone();
        let worker_model_path = PathBuf::from(&model_path);
        let worker_rx = rx.clone();
        tokio::spawn(async move {
            let worker = Worker::new(worker_id, worker_rx, worker_tasks, worker_model_path);
            worker.run().await;
        });
    }

    let app_state = web::Data::new(AppState {
        tasks: tasks.clone(),
        sender: tx,
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/", web::get().to(index))
            .route("/upload", web::post().to(upload))
            .route("/status/{id}", web::get().to(get_status))
            .route("/download/{id}", web::get().to(download_srt))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
