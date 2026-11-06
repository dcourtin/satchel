//! Free space of the filesystem holding a path.
use std::path::Path;

/// Bytes available to an unprivileged user (f_bavail × f_frsize), like `df`.
pub fn available_space(path: &Path) -> Option<u64> {
    let st = nix::sys::statvfs::statvfs(path).ok()?;
    Some(st.blocks_available() as u64 * st.fragment_size() as u64)
}

#[cfg(test)]
mod tests {
    #[test]
    fn root_has_space_info() {
        assert!(super::available_space(std::path::Path::new("/")).is_some());
    }
}
