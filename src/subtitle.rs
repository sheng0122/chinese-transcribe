use crate::TranscriptionSegment;
use std::fmt::Write;

/// Convert seconds to SRT timestamp format (HH:MM:SS,mmm)
fn format_timestamp(seconds: f32) -> String {
    let hours = (seconds / 3600.0) as u32;
    let minutes = ((seconds % 3600.0) / 60.0) as u32;
    let secs = (seconds % 60.0) as u32;
    let millis = ((seconds.fract()) * 1000.0) as u32;

    format!("{:02}:{:02}:{:02},{:03}", hours, minutes, secs, millis)
}

/// Generate SRT content from transcription segments
pub fn generate_srt(segments: &[TranscriptionSegment]) -> String {
    let mut output = String::new();

    for (i, segment) in segments.iter().enumerate() {
        let start = format_timestamp(segment.start);
        let end = format_timestamp(segment.end);
        
        // SRT index starts at 1
        writeln!(&mut output, "{}", i + 1).unwrap();
        writeln!(&mut output, "{} --> {}", start, end).unwrap();
        writeln!(&mut output, "{}", segment.text.trim()).unwrap();
        writeln!(&mut output).unwrap(); // Empty line after each block
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(0.0), "00:00:00,000");
        assert_eq!(format_timestamp(61.5), "00:01:01,500");
        assert_eq!(format_timestamp(3661.001), "01:01:01,001");
    }
}
