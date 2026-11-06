//! freedesktop Thumbnail Managing Standard: reuse thumbnails KDE already generated.
//! Path = $XDG_CACHE_HOME/thumbnails/{size}/md5(uri).png
use md5::{Digest, Md5};
use std::path::{Path, PathBuf};

/// file:// URI with RFC 3986 percent-encoding, as thumbnailers compute it.
pub fn file_uri(path: &Path) -> String {
    let mut out = String::from("file://");
    for b in path.as_os_str().as_encoded_bytes() {
        let c = *b;
        if c.is_ascii_alphanumeric() || b"/-._~!$&'()*+,;=:@".contains(&c) {
            out.push(c as char);
        } else {
            out.push_str(&format!("%{c:02X}"));
        }
    }
    out
}

fn cache_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .map(|c| c.join("thumbnails"))
}

pub fn hash_name(path: &Path) -> String {
    let digest = Md5::digest(file_uri(path).as_bytes());
    let mut s = String::with_capacity(36);
    for b in digest.iter() {
        s.push_str(&format!("{b:02x}"));
    }
    s.push_str(".png");
    s
}

/// Existing cached thumbnail (largest first). TODO: validate Thumb::MTime against the file.
pub fn cached(path: &Path) -> Option<PathBuf> {
    let dir = cache_dir()?;
    let name = hash_name(path);
    ["xx-large", "x-large", "large", "normal"].iter().map(|s| dir.join(s).join(&name)).find(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uri_encoding() {
        assert_eq!(file_uri(Path::new("/home/u/Mes Photos/é.jpg")), "file:///home/u/Mes%20Photos/%C3%A9.jpg");
    }

    #[test]
    fn known_hash() {
        // md5("file:///home/jens/photos/me.png") from the freedesktop spec example
        assert_eq!(hash_name(Path::new("/home/jens/photos/me.png")), "c6ee772d9e49320e97ec29a7eb5b1697.png");
    }
}
