use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// One item of a directory listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_symlink: bool,
    /// Size in bytes for files; number of children for directories (if readable).
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub hidden: bool,
}

impl Entry {
    /// Builds an entry from a path, following symlinks for type/size
    /// (a symlink to a directory behaves like a directory, as in Dolphin).
    pub fn from_path(path: &Path) -> io::Result<Self> {
        let link_meta = fs::symlink_metadata(path)?;
        let is_symlink = link_meta.file_type().is_symlink();
        // Broken symlink: keep the link's own metadata.
        let meta = if is_symlink { fs::metadata(path).unwrap_or(link_meta) } else { link_meta };
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        let is_dir = meta.is_dir();
        let size = if is_dir { count_children(path).unwrap_or(0) } else { meta.len() };
        Ok(Entry {
            hidden: name.starts_with('.'),
            name,
            path: path.to_path_buf(),
            is_dir,
            is_symlink,
            size,
            modified: meta.modified().ok(),
        })
    }
}

fn count_children(path: &Path) -> io::Result<u64> {
    Ok(fs::read_dir(path)?.count() as u64)
}

/// Lists a directory. Unreadable children are skipped instead of failing the whole listing.
pub fn list_dir(dir: &Path, show_hidden: bool) -> io::Result<Vec<Entry>> {
    let mut out = Vec::new();
    for item in fs::read_dir(dir)? {
        let Ok(item) = item else { continue };
        let path = item.path();
        let Ok(entry) = Entry::from_path(&path) else { continue };
        if entry.hidden && !show_hidden {
            continue;
        }
        out.push(entry);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_files_and_dirs_and_hides_dotfiles() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("a.txt"), b"hello").unwrap();
        fs::write(tmp.path().join(".hidden"), b"x").unwrap();
        fs::create_dir(tmp.path().join("sub")).unwrap();
        fs::write(tmp.path().join("sub/one"), b"").unwrap();

        let mut names: Vec<_> = list_dir(tmp.path(), false).unwrap().into_iter().map(|e| e.name).collect();
        names.sort();
        assert_eq!(names, ["a.txt", "sub"]);

        let all = list_dir(tmp.path(), true).unwrap();
        assert_eq!(all.len(), 3);
        let sub = all.iter().find(|e| e.name == "sub").unwrap();
        assert!(sub.is_dir);
        assert_eq!(sub.size, 1, "directory size = number of children");
        let a = all.iter().find(|e| e.name == "a.txt").unwrap();
        assert_eq!(a.size, 5);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_to_dir_is_a_dir_and_broken_link_is_listed() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("real")).unwrap();
        std::os::unix::fs::symlink(tmp.path().join("real"), tmp.path().join("link")).unwrap();
        std::os::unix::fs::symlink(tmp.path().join("nope"), tmp.path().join("broken")).unwrap();
        let all = list_dir(tmp.path(), false).unwrap();
        let link = all.iter().find(|e| e.name == "link").unwrap();
        assert!(link.is_dir && link.is_symlink);
        let broken = all.iter().find(|e| e.name == "broken").unwrap();
        assert!(!broken.is_dir && broken.is_symlink);
    }
}
