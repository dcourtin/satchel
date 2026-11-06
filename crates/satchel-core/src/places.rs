//! Sidebar sources: favorites from KDE's `user-places.xbel` (read-only for now, spec 04 §1.2)
//! and mounted devices from /proc/self/mounts.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub label: String,
    pub path: PathBuf,
    pub icon: String,
}

/// Reads local (file://) bookmarks, skipping hidden ones and app-restricted ones.
/// Minimal tolerant parser: XBEL written by KDE is regular; full XML handling comes with write support.
pub fn read_xbel(path: &Path) -> Vec<Place> {
    let Ok(xml) = std::fs::read_to_string(path) else { return Vec::new() };
    let mut out = Vec::new();
    for chunk in xml.split("<bookmark ").skip(1) {
        let block = chunk.split("</bookmark>").next().unwrap_or("");
        let Some(href) = attr(block, "href") else { continue };
        let Some(local) = href.strip_prefix("file://") else { continue };
        if block.contains("<IsHidden>true</IsHidden>") {
            continue;
        }
        if let Some(app) = tag(block, "OnlyInApp") {
            if !app.is_empty() && app != "satchel" {
                continue;
            }
        }
        let path = PathBuf::from(percent_decode(local));
        let label = tag(block, "title").map(|t| translate_system(&unescape(&t)).to_string()).unwrap_or_else(|| {
            path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "/".into())
        });
        let icon = icon_name(block).unwrap_or_else(|| "folder".into());
        out.push(Place { label, path, icon });
    }
    out
}

fn attr(block: &str, name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let start = block.find(&key)? + key.len();
    let end = block[start..].find('"')? + start;
    Some(block[start..end].to_string())
}

fn tag(block: &str, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let start = block.find(&open)? + open.len();
    let end = block[start..].find(&format!("</{name}>"))? + start;
    Some(block[start..end].to_string())
}

fn icon_name(block: &str) -> Option<String> {
    let start = block.find("<bookmark:icon ")?;
    attr(&block[start..], "name")
}

fn unescape(s: &str) -> String {
    s.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'")
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
            if let Ok(v) = u8::from_str_radix(hex, 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// KDE stores system places with English titles and translates at display time (spec 04 §1.2).
fn translate_system(title: &str) -> &str {
    match title {
        "Home" => "Dossier personnel",
        "Desktop" => "Bureau",
        "Documents" => "Documents",
        "Downloads" => "Téléchargements",
        "Music" => "Musique",
        "Pictures" => "Images",
        "Videos" => "Vidéos",
        "Trash" => "Corbeille",
        "Network" => "Réseau",
        "Root" => "Racine",
        other => other,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub label: String,
    pub mount_point: PathBuf,
    pub fs_type: String,
}

/// Real block-device mounts the user cares about (/, /home, /run/media/*, /mnt/*).
pub fn mounted_devices() -> Vec<Device> {
    let Ok(text) = std::fs::read_to_string("/proc/self/mounts") else { return Vec::new() };
    let mut out: Vec<Device> = Vec::new();
    for line in text.lines() {
        let mut f = line.split_whitespace();
        let (Some(src), Some(mp), Some(fs)) = (f.next(), f.next(), f.next()) else { continue };
        if !src.starts_with("/dev/") {
            continue;
        }
        let mp = mp.replace("\\040", " ");
        let interesting = mp == "/" || mp.starts_with("/run/media/") || mp.starts_with("/mnt/") || mp.starts_with("/media/");
        if !interesting || out.iter().any(|d| d.mount_point == Path::new(&mp)) {
            continue;
        }
        let label = if mp == "/" {
            "Système".to_string()
        } else {
            Path::new(&mp).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or(mp.clone())
        };
        out.push(Device { label, mount_point: PathBuf::from(mp), fs_type: fs.to_string() });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_kde_xbel() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("p.xbel");
        std::fs::write(
            &f,
            r#"<?xml version="1.0"?><xbel>
 <bookmark href="file:///home/u">
  <title>Home</title>
  <info><metadata owner="http://freedesktop.org"><bookmark:icon name="user-home"/></metadata>
  <metadata owner="http://www.kde.org"><isSystemItem>true</isSystemItem></metadata></info>
 </bookmark>
 <bookmark href="file:///home/u/Mes%20Projets"><title>Projets &amp; co</title></bookmark>
 <bookmark href="file:///tmp"><title>Tmp</title><info><metadata owner="http://www.kde.org"><IsHidden>true</IsHidden></metadata></info></bookmark>
 <bookmark href="remote:/"><title>Network</title></bookmark>
</xbel>"#,
        )
        .unwrap();
        let p = read_xbel(&f);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0], Place { label: "Dossier personnel".into(), path: "/home/u".into(), icon: "user-home".into() });
        assert_eq!(p[1].path, PathBuf::from("/home/u/Mes Projets"));
        assert_eq!(p[1].label, "Projets & co");
    }
}
