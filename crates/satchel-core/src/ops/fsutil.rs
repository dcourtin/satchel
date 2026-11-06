//! Low-level, data-safe filesystem primitives (spec 02 §17, "Exigences supplémentaires").
//! - never replace an existing name implicitly (`RENAME_NOREPLACE`);
//! - copies are written to `<name>.satchel-part`, fsynced, then renamed into place;
//! - recursive deletion never follows symlinks.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub const PART_SUFFIX: &str = ".satchel-part";
const CHUNK: usize = 16 * 1024 * 1024;

/// io::ErrorKind for "cancelled by the user".
pub fn cancelled() -> io::Error {
    io::Error::new(io::ErrorKind::Interrupted, "opération annulée")
}

/// rename(2) that fails with `AlreadyExists` instead of replacing the destination.
pub fn rename_noreplace(from: &Path, to: &Path) -> io::Result<()> {
    use rustix::fs::{CWD, RenameFlags, renameat_with};
    match renameat_with(CWD, from, CWD, to, RenameFlags::NOREPLACE) {
        Ok(()) => Ok(()),
        Err(e) if e == rustix::io::Errno::INVAL || e == rustix::io::Errno::NOSYS => {
            // Filesystem without RENAME_NOREPLACE support: check then rename (small race window).
            if fs::symlink_metadata(to).is_ok() {
                return Err(io::Error::new(io::ErrorKind::AlreadyExists, "la destination existe déjà"));
            }
            fs::rename(from, to)
        }
        Err(e) => Err(io::Error::from(e)),
    }
}

/// Atomically replaces `to` with `from` (used only after the user chose "Écraser" and the old
/// destination has been moved to the trash or explicitly accepted to be lost).
pub fn rename_replace(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)
}

pub fn same_device(a: &Path, b: &Path) -> bool {
    match (fs::symlink_metadata(a), fs::symlink_metadata(b)) {
        (Ok(x), Ok(y)) => x.dev() == y.dev(),
        _ => false,
    }
}

/// Temporary sibling path used while writing `dst`.
pub fn part_path(dst: &Path) -> PathBuf {
    let mut name = dst.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(PART_SUFFIX);
    dst.with_file_name(name)
}

/// Copies a regular file to `dst` (which must not exist): data → `dst.satchel-part`, fsync,
/// permissions and times preserved, then `rename_noreplace` into place. On any error or cancel the
/// partial file is removed and `dst` is untouched. `progress` receives bytes written per chunk.
pub fn copy_file_atomic(src: &Path, dst: &Path, cancel: &AtomicBool, progress: &mut dyn FnMut(u64)) -> io::Result<()> {
    let part = part_path(dst);
    let result = (|| {
        let mut input = File::open(src)?;
        let meta = input.metadata()?;
        let mut output = fs::OpenOptions::new().write(true).create_new(true).open(&part)?;
        copy_data(&mut input, &mut output, meta.len(), cancel, progress)?;
        output.set_permissions(fs::Permissions::from_mode(meta.permissions().mode()))?;
        let mut times = fs::FileTimes::new();
        if let Ok(m) = meta.modified() {
            times = times.set_modified(m);
        }
        if let Ok(a) = meta.accessed() {
            times = times.set_accessed(a);
        }
        output.set_times(times)?;
        output.sync_all()?;
        drop(output);
        rename_noreplace(&part, dst)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&part);
    }
    result
}

/// copy_file_range in chunks (reflink/server-side copy on btrfs etc.), falling back to read/write.
fn copy_data(input: &mut File, output: &mut File, len: u64, cancel: &AtomicBool, progress: &mut dyn FnMut(u64)) -> io::Result<()> {
    let mut copied: u64 = 0;
    let mut use_cfr = true;
    let mut buf = Vec::new();
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(cancelled());
        }
        if use_cfr {
            let want = CHUNK.min((len.saturating_sub(copied)).max(1) as usize);
            match rustix::fs::copy_file_range(&*input, None, &*output, None, want) {
                Ok(0) => {
                    // EOF (or a file that grew/shrank: fall back to read/write to be sure).
                    if copied >= len {
                        return Ok(());
                    }
                    use_cfr = false;
                }
                Ok(n) => {
                    copied += n as u64;
                    progress(n as u64);
                    continue;
                }
                Err(e) if copied == 0 && matches!(e, rustix::io::Errno::XDEV | rustix::io::Errno::INVAL | rustix::io::Errno::NOSYS | rustix::io::Errno::OPNOTSUPP) => {
                    use_cfr = false;
                }
                Err(e) => return Err(io::Error::from(e)),
            }
        }
        if buf.is_empty() {
            buf = vec![0u8; 1024 * 1024];
        }
        let n = input.read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }
        output.write_all(&buf[..n])?;
        copied += n as u64;
        progress(n as u64);
    }
}

/// Recreates a symlink (not its target) at `dst`, atomically.
pub fn copy_symlink(src: &Path, dst: &Path) -> io::Result<()> {
    let target = fs::read_link(src)?;
    let part = part_path(dst);
    std::os::unix::fs::symlink(&target, &part)?;
    let r = rename_noreplace(&part, dst);
    if r.is_err() {
        let _ = fs::remove_file(&part);
    }
    r
}

/// Creates a directory; fails with AlreadyExists rather than reusing one.
pub fn create_dir(path: &Path) -> io::Result<()> {
    fs::create_dir(path)
}

/// Removes a file, symlink or directory tree. Symlinks are removed, never followed
/// (`remove_dir_all` is symlink-safe since Rust 1.58.1, CVE-2022-21658).
pub fn remove_any(path: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.is_dir() { fs::remove_dir_all(path) } else { fs::remove_file(path) }
}

/// fsync a directory so that a rename inside it is durable before deleting a source.
pub fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

/// True if `inner` is `outer` or inside it (after resolving both). Used to refuse moving or
/// copying a folder into itself (spec 02 §17 #10).
pub fn is_same_or_inside(inner: &Path, outer: &Path) -> bool {
    match (fs::canonicalize(inner), fs::canonicalize(outer)) {
        (Ok(i), Ok(o)) => i.starts_with(o),
        _ => false,
    }
}

/// Total size in bytes of files under `path` (symlinks counted as 0, not followed).
pub fn tree_size(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else { return 0 };
    if meta.is_file() {
        return meta.len();
    }
    if !meta.is_dir() {
        return 0;
    }
    fs::read_dir(path)
        .map(|rd| rd.flatten().map(|e| tree_size(&e.path())).sum())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    #[test]
    fn rename_never_replaces() {
        let tmp = tempfile::tempdir().unwrap();
        let (a, b) = (tmp.path().join("a"), tmp.path().join("b"));
        fs::write(&a, "A").unwrap();
        fs::write(&b, "B").unwrap();
        let err = rename_noreplace(&a, &b).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(&b).unwrap(), "B");
        assert_eq!(fs::read_to_string(&a).unwrap(), "A");
    }

    #[test]
    fn atomic_copy_preserves_content_mode_mtime() {
        let tmp = tempfile::tempdir().unwrap();
        let (src, dst) = (tmp.path().join("s"), tmp.path().join("d"));
        let data: Vec<u8> = (0..3_000_000u32).map(|i| (i % 251) as u8).collect();
        fs::write(&src, &data).unwrap();
        fs::set_permissions(&src, fs::Permissions::from_mode(0o640)).unwrap();
        let old = SystemTime::now() - Duration::from_secs(86_400 * 30);
        File::options().write(true).open(&src).unwrap().set_modified(old).unwrap();

        let mut seen = 0;
        copy_file_atomic(&src, &dst, &AtomicBool::new(false), &mut |n| seen += n).unwrap();
        assert_eq!(fs::read(&dst).unwrap(), data);
        assert_eq!(seen, data.len() as u64);
        let m = fs::metadata(&dst).unwrap();
        assert_eq!(m.permissions().mode() & 0o777, 0o640);
        let diff = m.modified().unwrap().duration_since(old).unwrap_or_default();
        assert!(diff < Duration::from_secs(1));
        assert!(!part_path(&dst).exists());
    }

    #[test]
    fn atomic_copy_refuses_existing_and_leaves_no_part() {
        let tmp = tempfile::tempdir().unwrap();
        let (src, dst) = (tmp.path().join("s"), tmp.path().join("d"));
        fs::write(&src, "new").unwrap();
        fs::write(&dst, "old").unwrap();
        let err = copy_file_atomic(&src, &dst, &AtomicBool::new(false), &mut |_| {}).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(&dst).unwrap(), "old");
        assert!(!part_path(&dst).exists());
    }

    #[test]
    fn cancelled_copy_leaves_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let (src, dst) = (tmp.path().join("s"), tmp.path().join("d"));
        fs::write(&src, vec![1u8; 100]).unwrap();
        let err = copy_file_atomic(&src, &dst, &AtomicBool::new(true), &mut |_| {}).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Interrupted);
        assert!(!dst.exists() && !part_path(&dst).exists());
    }

    #[test]
    fn remove_any_does_not_follow_symlinks() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tmp.path().join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("precious"), "keep").unwrap();
        let tree = tmp.path().join("tree");
        fs::create_dir(&tree).unwrap();
        std::os::unix::fs::symlink(&outside, tree.join("link")).unwrap();
        remove_any(&tree).unwrap();
        assert!(!tree.exists());
        assert_eq!(fs::read_to_string(outside.join("precious")).unwrap(), "keep");
    }

    #[test]
    fn inside_detection() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a");
        fs::create_dir_all(a.join("b")).unwrap();
        assert!(is_same_or_inside(&a.join("b"), &a));
        assert!(is_same_or_inside(&a, &a));
        assert!(!is_same_or_inside(tmp.path(), &a));
    }
}
