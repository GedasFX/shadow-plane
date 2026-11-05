use serde::Deserialize;
use std::io;
use std::path::Path;
use std::process::Command;

#[derive(Debug, PartialEq)]
pub struct Probe {
    pub duration_s: f64,
    pub height: i64,
}

#[derive(Deserialize)]
struct Output {
    streams: Vec<Stream>,
    format: Format,
}

#[derive(Deserialize)]
struct Stream {
    height: i64,
}

#[derive(Deserialize)]
struct Format {
    duration: String,
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

// A video's length and height, or None when ffprobe can't read the file as a video.
pub fn probe(file: &Path) -> io::Result<Option<Probe>> {
    let mut ffprobe = Command::new("ffprobe");
    ffprobe
        .args(["-v", "error", "-select_streams", "v:0"])
        .args([
            "-show_entries",
            "stream=height:format=duration",
            "-of",
            "json",
        ])
        .arg(file);

    // A windowed app's console child opens a console window of its own unless told not to.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        ffprobe.creation_flags(CREATE_NO_WINDOW);
    }

    let output = match ffprobe.output() {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(io::Error::new(e.kind(), "FFmpeg isn't installed"));
        }
        result => result?,
    };

    Ok(parse(&output.stdout))
}

// The output decides, not the exit code: a file ffprobe fails on prints no video stream.
fn parse(json: &[u8]) -> Option<Probe> {
    let output: Output = serde_json::from_slice(json).ok()?;
    let video = output.streams.first()?;
    let duration_s = output.format.duration.parse().ok()?;

    Some(Probe {
        duration_s,
        height: video.height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_length_and_height_ffprobe_reports() {
        let json = br#"{
            "programs": [],
            "stream_groups": [],
            "streams": [{ "height": 1440 }],
            "format": { "duration": "150.267133" }
        }"#;
        let probe = Probe {
            duration_s: 150.267133,
            height: 1440,
        };
        assert_eq!(parse(json), Some(probe));
    }

    #[test]
    fn a_file_without_a_video_is_unreadable() {
        let no_video = br#"{ "streams": [], "format": { "duration": "3.0" } }"#;
        assert_eq!(parse(no_video), None);
        assert_eq!(parse(b""), None);
    }
}
