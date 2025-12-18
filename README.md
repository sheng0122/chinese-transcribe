# Transcribe-RS (高效能中文語音轉錄工具)

這是一個基於 Rust 開發的高效能並行語音轉錄工具，專為 **macOS (Apple Silicon)** 優化，使用 OpenAI 的 Whisper 引擎進行語音轉字。

## 特色

- **極速轉錄**：利用多進程架構 (Multi-process) 並行處理長錄音檔，大幅縮短轉錄時間。
- **針對 macOS 優化**：支援 Metal (GPU) 加速，並透過進程隔離解決 Metal 在多執行緒下的穩定性問題。
- **智慧分段**：自動將長錄音檔分割為 5 分鐘的小段進行處理，並包含重疊區間以確保上下文連貫。
- **自動清理**：內建幻覺 (Hallucination) 清理功能，自動過濾常見的無意義重複語句。

## 系統需求

在安裝之前，請確保您的系統已安裝以下工具：

1. **Rust Toolchain**：請確保已安裝 Rust ([安裝教學](https://www.rust-lang.org/tools/install))。
2. **FFmpeg**：用於音訊轉換。
   ```bash
   brew install ffmpeg
   ```

## 安裝方法

### 自動安裝 (推薦)

我們提供了一個安裝腳本，可以自動編譯並將執行檔安裝到 `/usr/local/bin`。

```bash
./install.sh
```

### 手動安裝

如果您偏好手動安裝，請執行以下指令：

```bash
# 編譯 CLI 工具與 Worker
cargo build --release --bin cli_tool
cargo build --release --bin worker

# (可選) 將執行檔移至系統路徑，例如 /usr/local/bin
# 需將 cli_tool 重新命名為 transcribe，worker 重新命名為 transcribe-worker
sudo cp target/release/cli_tool /usr/local/bin/transcribe
sudo cp target/release/worker /usr/local/bin/transcribe-worker
```

## 模型下載與設定

本工具需要 Whisper 的 GGML 格式模型才能運作。

1. **推薦模型 (繁體中文優化)**：
   我們推薦使用 MediaTek Research 開發的 Breeze (微風) 模型，針對中文有更好的表現。

   - **檔名**：`breeze-asr-25-q4_k.bin` (或其他支援的 GGML 格式模型)
   - **下載位置**：[Hugging Face - Breeze ASR 25](https://huggingface.co/alan314159/Breeze-ASR-25-whispercpp/tree/main)
2. **放置位置**：
   請將下載的模型檔案放入專案根目錄下的 `models/` 資料夾中。

   ```bash
   mkdir -p models
   # 將模型檔案放至 models/breeze-asr-25-q4_k.bin
   ```

   *注意：如果您的模型檔名不同，請在執行時指定模型路徑。*

## 使用方法

### 基本指令

安裝完成後，您可以使用 `transcribe` 指令來轉錄音訊檔案 (支援 mp3, wav, m4a 等格式)。

```bash
# 使用預設模型 (models/breeze-asr-25-q4_k.bin) 轉錄
transcribe /path/to/your/audio.mp3
```

### 指定模型

如果您想使用其他模型，可以在第二個參數指定模型路徑：

```bash
transcribe /path/to/your/audio.mp3 /path/to/custom_model.bin
```

### 輸出結果

轉錄完成後，工具會自動產生以下檔案與資料夾：

- **outputs/**：存放轉錄完成的 SRT 字幕檔與 TXT 純文字檔。
- **completed/**：轉錄成功的原始音訊檔案會被自動移動到這裡，方便檔案管理。

### 檔案結構範例

```text
.
├── models/
│   └── breeze-asr-25-q4_k.bin  <-- 您的模型檔案
├── outputs/
│   ├── audio.srt               <-- 字幕檔
│   └── audio.txt               <-- 純文字檔
├── completed/
│   └── audio.mp3               <-- 處理完的音檔
└── ...
```

## 開發者指南

如果您想參與開發或進行測試：

- **啟動 API Server**：

  ```bash
  cargo run --bin server
  ```

  這將會在 Port 8080 啟動一個 HTTP 伺服器，支援上傳轉錄。
- **執行測試**：

  ```bash
  cargo test
  ```

---

Powered by [whisper.rs](https://github.com/tazz4843/whisper-rs) and [burn](https://github.com/tracel-ai/burn) (Parakeet support).
