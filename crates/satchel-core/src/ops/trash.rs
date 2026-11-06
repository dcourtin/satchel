//! freedesktop.org Trash specification 1.0, implemented directly so that:
//! - an item is only ever *renamed* into a trash on its own filesystem (never copied across
//!   devices into the home trash — spec 02 §8 / §17 #3);
//! - the caller gets back exactly where the item went, for undo/restore.

use super::fsutil::rename_noreplace;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

/// Where an item went, enough to restore it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trashed {
    pub original: PathBuf,
    pub file: PathBuf,
    pub info: PathBuf,
}

#[derive(Debug)]
pub enum TrashError {
    /// No usable trash on this item's filesystem (read-only, network, FAT…): the UI must ask
    /// whether to delete permanently instead.
    Unavailable,
    Io(io::Error),
}

impl std::fmt::Display for TrashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrashError::Unavailable => write!(f, "aucune corbeille n'est disponible sur ce disque"),
            TrashError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl From<io::Error> for TrashError {
    fn from(e: io::Error) -> Self {
        TrashError::Io(e)
    }
}

/// Location of the user's home trash ($XDG_DATA_HOME/Trash). Injectable for tests.
#[derive(Debug, Clone)]
pub struct TrashConfig {
    pub home_trash: PathBuf,
}

impl TrashConfig {
    pub fn from_env() -> Self {
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share"));
        TrashConfig { home_trash: data.join("Trash") }
    }
}

struct TrashDir {
    root: PathBuf,
    /// Some(topdir) for $topdir trashes: Path= is then stored relative to topdir.
    topdir: Option<PathBuf>,
}

fn dev_of(p: &Path) -> io::Result<u64> {
    Ok(fs::symlink_metadata(p)?.dev())
}

fn uid() -> u32 {
    rustix::process::getuid().as_raw()
}

/// Mount point of `path`'s filesystem: highest ancestor on the same device.
fn topdir_of(path: &Path, dev: u64) -> PathBuf {
    let mut top = path.to_path_buf();
    let mut cur = path.parent();
    while let Some(p) = cur {
        match dev_of(p) {
            Ok(d) if d == dev => top = p.to_path_buf(),
            _ => break,
        }
        cur = p.parent();
    }
    top
}

fn ensure_dirs(root: &Path) -> io::Result<()> {
    for sub in ["files", "info"] {
        let d = root.join(sub);
        if !d.is_dir() {
            fs::create_dir_all(&d)?;
            fs::set_permissions(root, fs::Permissions::from_mode(0o700))?;
        }
    }
    Ok(())
}

fn trash_dir_for(item: &Path, cfg: &TrashConfig) -> Result<TrashDir, TrashError> {
    // The item's own metadata (lstat): a symlink lives on its parent's filesystem.
    let dev = dev_of(item)?;
    // 1. Home trash if it is on the same filesystem.
    let home_parent = cfg.home_trash.parent().unwrap_or(Path::new("/"));
    if fs::create_dir_all(home_parent).is_ok() && dev_of(home_parent).ok() == Some(dev) {
        ensure_dirs(&cfg.home_trash)?;
        if dev_of(&cfg.home_trash)? == dev {
            return Ok(TrashDir { root: cfg.home_trash.clone(), topdir: None });
        }
    }
    // 2. $topdir/.Trash/$uid if $topdir/.Trash is a real sticky directory (not a symlink).
    let parent = item.parent().unwrap_or(Path::new("/"));
    let top = topdir_of(parent, dev);
    let shared = top.join(".Trash");
    if let Ok(m) = fs::symlink_metadata(&shared) {
        if m.is_dir() && m.permissions().mode() & 0o1000 != 0 {
            let root = shared.join(uid().to_string());
            if ensure_dirs(&root).is_ok() && dev_of(&root).ok() == Some(dev) {
                return Ok(TrashDir { root, topdir: Some(top) });
            }
        }
    }
    // 3. $topdir/.Trash-$uid.
    let root = top.join(format!(".Trash-{}", uid()));
    if ensure_dirs(&root).is_ok() && dev_of(&root).ok() == Some(dev) {
        return Ok(TrashDir { root, topdir: Some(top) });
    }
    Err(TrashError::Unavailable)
}

/// Percent-encodes a path for the `Path=` key (RFC 2396 escaping, '/' kept).
fn encode_path(p: &Path) -> String {
    let mut out = String::new();
    for &b in p.as_os_str().as_encoded_bytes() {
        if b.is_ascii_alphanumeric() || b"/-._~!$&'()*+,;=:@".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn decode_path(s: &str) -> PathBuf {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if let Some(v) = bytes.get(i + 1..i + 3).and_then(|h| std::str::from_utf8(h).ok()).and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    use std::os::unix::ffi::OsStringExt;
    PathBuf::from(std::ffi::OsString::from_vec(out))
}

/// Moves `item` to the trash of its filesystem. The `.trashinfo` is written first (create_new
/// reserves the name), then the item is renamed into `files/`; on failure the info is removed.
pub fn trash(item: &Path, cfg: &TrashConfig) -> Result<Trashed, TrashError> {
    let original = if item.is_absolute() { item.to_path_buf() } else { std::env::current_dir()?.join(item) };
    let dir = trash_dir_for(&original, cfg)?;
    let name = original.file_name().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "chemin sans nom"))?.to_string_lossy().into_owned();
    let stored_path = match &dir.topdir {
        Some(top) => original.strip_prefix(top).map(Path::to_path_buf).unwrap_or_else(|_| original.clone()),
        None => original.clone(),
    };
    let body = format!(
        "[Trash Info]\nPath={}\nDeletionDate={}\n",
        encode_path(&stored_path),
        crate::format::iso_datetime(std::time::SystemTime::now())
    );
    for n in 0u32.. {
        let candidate = if n == 0 { name.clone() } else { format!("{name}.{n}") };
        let info = dir.root.join("info").join(format!("{candidate}.trashinfo"));
        let file = dir.root.join("files").join(&candidate);
        let mut f = match fs::OpenOptions::new().write(true).create_new(true).open(&info) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        };
        if fs::symlink_metadata(&file).is_ok() {
            // Orphan in files/ without info: pick another name.
            drop(f);
            let _ = fs::remove_file(&info);
            continue;
        }
        let written = f.write_all(body.as_bytes()).and_then(|_| f.sync_all());
        drop(f);
        if let Err(e) = written.and_then(|_| rename_noreplace(&original, &file)) {
            let _ = fs::remove_file(&info);
            return Err(e.into());
        }
        return Ok(Trashed { original, file, info });
    }
    unreachable!()
}

/// Puts a trashed item back. Refuses (without overwriting) if the original folder is gone or the
/// name is taken (spec 02 §17 #20).
pub fn restore(t: &Trashed) -> io::Result<()> {
    let parent = t.original.parent().unwrap_or(Path::new("/"));
    if !parent.is_dir() {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("le dossier d'origine {} n'existe plus", parent.display())));
    }
    rename_noreplace(&t.file, &t.original)?;
    let _ = fs::remove_file(&t.info);
    Ok(())
}

/// Reads the original path stored in a `.trashinfo` (resolving $topdir-relative paths).
pub fn read_info_path(info: &Path) -> io::Result<PathBuf> {
    let text = fs::read_to_string(info)?;
    let raw = text.lines().find_map(|l| l.strip_prefix("Path=")).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Path= manquant"))?;
    let p = decode_path(raw.trim());
    if p.is_absolute() {
        return Ok(p);
    }
    // $topdir/.Trash/$uid/info/x or $topdir/.Trash-$uid/info/x → topdir is 3 or 2 levels up.
    let root = info.parent().and_then(Path::parent).unwrap_or(Path::new("/"));
    let top = if root.parent().and_then(|p| p.file_name()).is_some_and(|n| n == ".Trash") { root.parent().and_then(Path::parent) } else { root.parent() };
    Ok(top.unwrap_or(Path::new("/")).join(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(tmp: &Path) -> TrashConfig {
        TrashConfig { home_trash: tmp.join("data/Trash") }
    }

    #[test]
    fn trash_and_restore_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let c = cfg(tmp.path());
        let f = tmp.path().join("doc été.txt");
        fs::write(&f, "contenu").unwrap();

        let t = trash(&f, &c).unwrap();
        assert!(!f.exists());
        assert_eq!(fs::read_to_string(&t.file).unwrap(), "contenu");
        let info = fs::read_to_string(&t.info).unwrap();
        assert!(info.starts_with("[Trash Info]\nPath=/"));
        assert!(info.contains("doc%20%C3%A9t%C3%A9.txt"));
        assert!(info.contains("DeletionDate=20"));
        assert_eq!(read_info_path(&t.info).unwrap(), f);

        restore(&t).unwrap();
        assert_eq!(fs::read_to_string(&f).unwrap(), "contenu");
        assert!(!t.info.exists() && !t.file.exists());
    }

    #[test]
    fn same_name_twice_gets_unique_slots() {
        let tmp = tempfile::tempdir().unwrap();
        let c = cfg(tmp.path());
        let f = tmp.path().join("a");
        fs::write(&f, "1").unwrap();
        let t1 = trash(&f, &c).unwrap();
        fs::write(&f, "2").unwrap();
        let t2 = trash(&f, &c).unwrap();
        assert_ne!(t1.file, t2.file);
        assert_eq!(fs::read_to_string(&t1.file).unwrap(), "1");
        assert_eq!(fs::read_to_string(&t2.file).unwrap(), "2");
    }

    #[test]
    fn restore_refuses_taken_name() {
        let tmp = tempfile::tempdir().unwrap();
        let c = cfg(tmp.path());
        let f = tmp.path().join("a");
        fs::write(&f, "old").unwrap();
        let t = trash(&f, &c).unwrap();
        fs::write(&f, "new").unwrap();
        assert_eq!(restore(&t).unwrap_err().kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(&f).unwrap(), "new");
        assert!(t.file.exists(), "trashed copy must stay in the trash");
    }

    #[test]
    fn trash_directory_and_symlink() {
        let tmp = tempfile::tempdir().unwrap();
        let c = cfg(tmp.path());
        let d = tmp.path().join("dir");
        fs::create_dir_all(d.join("sub")).unwrap();
        fs::write(d.join("sub/x"), "x").unwrap();
        let t = trash(&d, &c).unwrap();
        assert_eq!(fs::read_to_string(t.file.join("sub/x")).unwrap(), "x");
        let target = tmp.path().join("target");
        fs::write(&target, "t").unwrap();
        let l = tmp.path().join("link");
        std::os::unix::fs::symlink(&target, &l).unwrap();
        let tl = trash(&l, &c).unwrap();
        assert!(fs::symlink_metadata(&tl.file).unwrap().file_type().is_symlink());
        assert_eq!(fs::read_to_string(&target).unwrap(), "t", "symlink target untouched");
    }

    #[test]
    fn path_encoding_roundtrip() {
        let p = Path::new("/home/u/Mes Photos/été #1.jpg");
        assert_eq!(decode_path(&encode_path(p)), p);
    }
}
