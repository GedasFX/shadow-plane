use crate::{bin, library};
use percent_encoding::percent_decode_str;
use std::fs::{self, File};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use tauri::http::{Request, Response, StatusCode, header};

// The asset protocol's chunk, which the PoC's seek test measured over the NAS.
const CHUNK: u64 = 1000 * 1024;

// One ffmpeg at a time: a grid asks for many thumbnails at once.
static MAKING: Mutex<()> = Mutex::new(());

// Answers the media protocol: "video/<key>" in ranges, "thumb/<key>" as a JPEG made on first use.
pub fn respond<F>(
    root: Option<&Path>,
    cache: &Path,
    request: &Request<Vec<u8>>,
    make: F,
) -> Response<Vec<u8>>
where
    F: Fn(&Path, &Path) -> io::Result<()>,
{
    let Some(root) = root else {
        return status(StatusCode::NOT_FOUND, Vec::new());
    };

    let path = percent_decode_str(request.uri().path().trim_start_matches('/'));
    let path = path.decode_utf8_lossy();

    let range = request.headers().get(header::RANGE);
    let range = range.and_then(|range| range.to_str().ok());

    let Some((kind, key)) = path.split_once('/') else {
        return status(StatusCode::NOT_FOUND, Vec::new());
    };
    let Some(file) = below(root, key) else {
        return status(StatusCode::FORBIDDEN, Vec::new());
    };
    let file = in_bin_if_gone(root, key, file);

    let served = match kind {
        "video" => video(&file, range),
        "thumb" => thumbnail(&file, key, cache, make),
        _ => return status(StatusCode::NOT_FOUND, Vec::new()),
    };

    served.unwrap_or_else(|e| {
        let code = match e.kind() {
            io::ErrorKind::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        status(code, e.to_string().into_bytes())
    })
}

fn status(code: StatusCode, body: Vec<u8>) -> Response<Vec<u8>> {
    let mut response = Response::new(body);
    *response.status_mut() = code;

    response
}

// A binned video is served from _Bin, under the key it had before.
fn in_bin_if_gone(root: &Path, key: &str, file: PathBuf) -> PathBuf {
    if file.exists() {
        return file;
    }

    below(&root.join(bin::FOLDER), key)
        .filter(|b| b.exists())
        .unwrap_or(file)
}

// The file a key names, if the key stays below the library folder.
pub fn below(root: &Path, key: &str) -> Option<PathBuf> {
    let mut parts = Path::new(key).components();
    let plain = parts.all(|part| matches!(part, Component::Normal(_)));

    (plain && !key.is_empty()).then(|| root.join(key))
}

// Always a part of the file: a source runs to 15 GB, and the page asks for more as it plays.
fn video(file: &Path, range: Option<&str>) -> io::Result<Response<Vec<u8>>> {
    let mut file = File::open(file)?;
    let len = file.metadata()?.len();
    let Some((first, last)) = span(range, len) else {
        let refused = Response::builder()
            .status(StatusCode::RANGE_NOT_SATISFIABLE)
            .header(header::CONTENT_RANGE, format!("bytes */{len}"));
        return refused.body(Vec::new()).map_err(io::Error::other);
    };

    let mut bytes = Vec::with_capacity((last + 1 - first) as usize);
    file.seek(SeekFrom::Start(first))?;
    file.take(last + 1 - first).read_to_end(&mut bytes)?;

    let partial = Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header(header::CONTENT_TYPE, "video/mp4")
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_RANGE, format!("bytes {first}-{last}/{len}"))
        .header(header::CONTENT_LENGTH, bytes.len());
    partial.body(bytes).map_err(io::Error::other)
}

// The first and last byte to send, at most a chunk; None when the range starts past the end.
fn span(range: Option<&str>, len: u64) -> Option<(u64, u64)> {
    let (first, last) = asked(range, len).unwrap_or((0, u64::MAX));
    if first >= len {
        return None;
    }

    Some((first, last.min(len - 1).min(first + CHUNK - 1)))
}

// "bytes=100-199", "bytes=100-" or the last 500 bytes, "bytes=-500"; None for anything else.
fn asked(range: Option<&str>, len: u64) -> Option<(u64, u64)> {
    let (first, last) = range?.strip_prefix("bytes=")?.split_once('-')?;
    if first.is_empty() {
        let suffix: u64 = last.parse().ok()?;
        return Some((len.saturating_sub(suffix), u64::MAX));
    }

    let first: u64 = first.parse().ok()?;
    let last = if last.is_empty() {
        u64::MAX
    } else {
        last.parse().ok()?
    };

    (first <= last).then_some((first, last))
}

fn thumbnail<F>(video: &Path, key: &str, cache: &Path, make: F) -> io::Result<Response<Vec<u8>>>
where
    F: Fn(&Path, &Path) -> io::Result<()>,
{
    let meta = fs::metadata(video)?;
    let name = thumb_name(key, meta.len(), library::modified_ms(&meta)?);
    let jpeg = cache.join(name);

    if !jpeg.exists() {
        let _one_at_a_time = MAKING.lock().unwrap();
        // Another request may have made it while this one waited.
        if !jpeg.exists() {
            // Made under another name first, so a failed ffmpeg leaves no half thumbnail.
            let part = jpeg.with_extension("part.jpg");
            fs::create_dir_all(cache)?;
            make(video, &part)?;
            fs::rename(part, &jpeg)?;
        }
    }

    let bytes = fs::read(&jpeg)?;
    let found = Response::builder().header(header::CONTENT_TYPE, "image/jpeg");

    found.body(bytes).map_err(io::Error::other)
}

// A thumbnail's file in the cache; a changed video gets a new one. A new Rust may hash
// differently, which only makes the thumbnails again.
fn thumb_name(key: &str, size: u64, modified_ms: i64) -> String {
    let mut hash = DefaultHasher::new();
    (key, size, modified_ms).hash(&mut hash);

    format!("{:016x}.jpg", hash.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use tempfile::TempDir;

    fn library() -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("library/PEAK")).unwrap();
        fs::write(dir.path().join("library/PEAK/a b.mp4"), b"0123456789").unwrap();
        fs::write(dir.path().join("secret.txt"), b"secret").unwrap();
        dir
    }

    fn get(path: &str, range: Option<&str>) -> Request<Vec<u8>> {
        let mut request = Request::builder().uri(format!("http://media.localhost/{path}"));
        if let Some(range) = range {
            request = request.header(header::RANGE, range);
        }
        request.body(Vec::new()).unwrap()
    }

    fn no_ffmpeg(_: &Path, _: &Path) -> io::Result<()> {
        panic!("no thumbnail asked for")
    }

    #[test]
    fn a_video_is_served_in_the_range_asked_for() {
        let dir = library();
        let root = dir.path().join("library");
        let request = get("video%2FPEAK%2Fa%20b.mp4", Some("bytes=2-5"));

        let response = respond(Some(&root), dir.path(), &request, no_ffmpeg);

        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.headers()[header::CONTENT_RANGE], "bytes 2-5/10");
        assert_eq!(response.headers()[header::CONTENT_TYPE], "video/mp4");
        assert_eq!(response.body(), b"2345");
    }

    #[test]
    fn a_binned_video_and_its_thumbnail_are_served_from_bin() {
        let dir = library();
        let root = dir.path().join("library");
        fs::create_dir_all(root.join("_Bin/PEAK")).unwrap();
        fs::rename(root.join("PEAK/a b.mp4"), root.join("_Bin/PEAK/a b.mp4")).unwrap();
        let make = |_: &Path, jpeg: &Path| fs::write(jpeg, b"jpeg");

        let video = get("video%2FPEAK%2Fa%20b.mp4", Some("bytes=2-5"));
        let video = respond(Some(&root), dir.path(), &video, no_ffmpeg);
        let thumb = get("thumb%2FPEAK%2Fa%20b.mp4", None);
        let thumb = respond(Some(&root), &dir.path().join("cache"), &thumb, make);

        assert_eq!(video.body(), b"2345");
        assert_eq!(thumb.status(), StatusCode::OK);
    }

    #[test]
    fn a_range_past_the_end_is_refused() {
        let dir = library();
        let root = dir.path().join("library");
        let request = get("video%2FPEAK%2Fa%20b.mp4", Some("bytes=10-"));

        let response = respond(Some(&root), dir.path(), &request, no_ffmpeg);

        assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);
        assert_eq!(response.headers()[header::CONTENT_RANGE], "bytes */10");
    }

    #[test]
    fn nothing_outside_the_library_is_served() {
        let dir = library();
        let root = dir.path().join("library");
        let status = |path| respond(Some(&root), dir.path(), &get(path, None), no_ffmpeg).status();

        assert_eq!(status("video%2F..%2Fsecret.txt"), StatusCode::FORBIDDEN);
        assert_eq!(status("video%2FPEAK%2Fmissing.mp4"), StatusCode::NOT_FOUND);
        assert_eq!(status("other%2FPEAK%2Fa%20b.mp4"), StatusCode::NOT_FOUND);
        let unset = respond(
            None,
            dir.path(),
            &get("video%2FPEAK%2Fa%20b.mp4", None),
            no_ffmpeg,
        );
        assert_eq!(unset.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn a_key_must_stay_below_the_library_folder() {
        let root = Path::new("library");

        assert_eq!(below(root, "PEAK/a.mp4"), Some(root.join("PEAK/a.mp4")));
        assert_eq!(below(root, "a.mp4"), Some(root.join("a.mp4")));
        for key in ["", "..", "../a.mp4", "PEAK/../../a.mp4", "/etc/passwd"] {
            assert_eq!(below(root, key), None, "{key}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn a_key_cannot_name_a_drive_or_a_share() {
        let root = Path::new(r"Z:\library");

        for key in [
            r"C:\Windows\win.ini",
            "C:a.mp4",
            r"\\server\share\a.mp4",
            r"PEAK\..\..\a.mp4",
        ] {
            assert_eq!(below(root, key), None, "{key}");
        }
    }

    #[test]
    fn a_range_is_read_from_the_header_and_capped_at_a_chunk() {
        let big = 10 * CHUNK;

        assert_eq!(span(Some("bytes=100-199"), 1000), Some((100, 199)));
        assert_eq!(span(Some("bytes=100-"), 150), Some((100, 149)));
        assert_eq!(span(Some("bytes=100-5000"), 150), Some((100, 149)));
        assert_eq!(span(Some("bytes=0-"), big), Some((0, CHUNK - 1)));
        assert_eq!(span(Some("bytes=-50"), 1000), Some((950, 999)));
        assert_eq!(span(Some("bytes=-5000"), 1000), Some((0, 999)));
        assert_eq!(span(Some("bytes=1000-"), 1000), None);
        assert_eq!(span(Some("bytes=-0"), 1000), None);
    }

    #[test]
    fn a_missing_or_unread_range_starts_at_the_beginning() {
        for range in [
            None,
            Some("pages=1-2"),
            Some("bytes=5-2"),
            Some("bytes=0-1,4-5"),
        ] {
            assert_eq!(span(range, 1000), Some((0, 999)), "{range:?}");
        }
    }

    #[test]
    fn a_thumbnail_is_named_by_the_videos_key_size_and_modified_time() {
        let name = thumb_name("PEAK/a.mp4", 10, 1000);

        assert_eq!(name, thumb_name("PEAK/a.mp4", 10, 1000));
        assert!(name.ends_with(".jpg"));
        assert_ne!(name, thumb_name("PEAK/b.mp4", 10, 1000));
        assert_ne!(name, thumb_name("PEAK/a.mp4", 11, 1000));
        assert_ne!(name, thumb_name("PEAK/a.mp4", 10, 1001));
    }

    #[test]
    fn a_thumbnail_is_made_once_for_each_version_of_a_video() {
        let dir = library();
        let root = dir.path().join("library");
        let cache = dir.path().join("cache");
        let made = Cell::new(0);
        let make = |_: &Path, jpeg: &Path| {
            made.set(made.get() + 1);
            fs::write(jpeg, b"jpeg")
        };
        let request = get("thumb%2FPEAK%2Fa%20b.mp4", None);

        let first = respond(Some(&root), &cache, &request, make);
        respond(Some(&root), &cache, &request, make);

        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(first.headers()[header::CONTENT_TYPE], "image/jpeg");
        assert_eq!(first.body(), b"jpeg");
        assert_eq!(made.get(), 1);

        fs::write(root.join("PEAK/a b.mp4"), b"longer video").unwrap();
        respond(Some(&root), &cache, &request, make);

        assert_eq!(made.get(), 2);
    }
}
