use sha1::{Digest, Sha1};
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::Path;

// Renames; when the target is on another volume, copies, compares and then deletes the original.
// Never overwrites: a target that exists is an error.
pub fn move_file(from: &Path, to: &Path) -> io::Result<()> {
    if to.exists() {
        return Err(exists(to));
    }

    match fs::rename(from, to) {
        Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
            copy_verified(from, to)?;
            fs::remove_file(from)
        }
        done => done,
    }
}

// Copies a file to a new name, then reads both back; a copy that differs is deleted and is an error.
pub fn copy_verified(from: &Path, to: &Path) -> io::Result<()> {
    let mut source = File::open(from)?;
    let mut target = OpenOptions::new().write(true).create_new(true).open(to)?;
    let written = io::copy(&mut source, &mut target).and_then(|_| target.sync_all());
    drop(target);
    if let Err(e) = written {
        let _ = fs::remove_file(to);
        return Err(e);
    }

    if sha1_of(from)? != sha1_of(to)? {
        fs::remove_file(to)?;
        return Err(io::Error::other(format!(
            "The copy of {} differs from the original",
            from.display()
        )));
    }

    Ok(())
}

fn sha1_of(path: &Path) -> io::Result<Vec<u8>> {
    let mut hash = Sha1::new();
    io::copy(&mut File::open(path)?, &mut hash)?;

    Ok(hash.finalize().to_vec())
}

fn exists(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("{} already exists", path.display()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_move_renames_the_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.mp4"), b"video").unwrap();

        move_file(&dir.path().join("a.mp4"), &dir.path().join("b.mp4")).unwrap();

        assert!(!dir.path().join("a.mp4").exists());
        assert_eq!(fs::read(dir.path().join("b.mp4")).unwrap(), b"video");
    }

    #[test]
    fn a_move_never_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.mp4"), b"new").unwrap();
        fs::write(dir.path().join("b.mp4"), b"old").unwrap();

        let refused = move_file(&dir.path().join("a.mp4"), &dir.path().join("b.mp4"));

        assert_eq!(refused.unwrap_err().kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(dir.path().join("a.mp4")).unwrap(), b"new");
        assert_eq!(fs::read(dir.path().join("b.mp4")).unwrap(), b"old");
    }

    #[test]
    fn a_verified_copy_leaves_the_original_and_never_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.mp4"), b"video").unwrap();

        copy_verified(&dir.path().join("a.mp4"), &dir.path().join("b.mp4")).unwrap();
        let again = copy_verified(&dir.path().join("a.mp4"), &dir.path().join("b.mp4"));

        assert_eq!(fs::read(dir.path().join("a.mp4")).unwrap(), b"video");
        assert_eq!(fs::read(dir.path().join("b.mp4")).unwrap(), b"video");
        assert_eq!(again.unwrap_err().kind(), io::ErrorKind::AlreadyExists);
    }

    #[test]
    fn a_missing_original_leaves_no_copy_behind() {
        let dir = tempfile::tempdir().unwrap();

        let failed = copy_verified(&dir.path().join("none.mp4"), &dir.path().join("b.mp4"));

        assert_eq!(failed.unwrap_err().kind(), io::ErrorKind::NotFound);
        assert!(!dir.path().join("b.mp4").exists());
    }
}
