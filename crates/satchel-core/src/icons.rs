//! File icon resolution following freedesktop specs: shared-mime-info (type by file name) and
//! the Icon Theme spec (lookup in the user's KDE icon theme, with inheritance).
//! Returns icon file paths only; loading/rendering is the UI's job.

use crate::Entry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use xdg_mime::SharedMimeInfo;

pub struct IconResolver {
    mime: SharedMimeInfo,
    theme: String,
    home: PathBuf,
    /// XDG user directories (Pictures, Music…) → their dedicated folder icon.
    special_dirs: Vec<(PathBuf, &'static str)>,
    cache: HashMap<(String, u16), Option<PathBuf>>,
}

impl IconResolver {
    pub fn new(home: &Path) -> Self {
        IconResolver {
            mime: SharedMimeInfo::new(),
            theme: current_theme(home),
            home: home.to_path_buf(),
            special_dirs: user_dirs(home),
            cache: HashMap::new(),
        }
    }

    pub fn theme(&self) -> &str {
        &self.theme
    }

    /// Candidate icon names, most specific first.
    pub fn icon_names(&self, e: &Entry) -> Vec<String> {
        if e.is_dir {
            let mut names = Vec::new();
            if e.path == self.home {
                names.push("user-home".to_string());
            }
            if let Some((_, icon)) = self.special_dirs.iter().find(|(p, _)| *p == e.path) {
                names.push((*icon).to_string());
            }
            names.push("folder".to_string());
            return names;
        }
        let mut names: Vec<String> = self
            .mime
            .get_mime_types_from_file_name(&e.name)
            .first()
            .map(|m| self.mime.lookup_icon_names(m))
            .unwrap_or_default();
        names.push("text-x-generic".to_string());
        names.push("unknown".to_string());
        names
    }

    /// Path of the best icon at `size` px, cached per (name, size).
    pub fn resolve(&mut self, e: &Entry, size: u16) -> Option<PathBuf> {
        for name in self.icon_names(e) {
            let key = (name, size);
            let found = match self.cache.get(&key) {
                Some(hit) => hit.clone(),
                None => {
                    let found = freedesktop_icons::lookup(&key.0).with_size(size).with_theme(&self.theme).with_cache().find();
                    self.cache.insert(key, found.clone());
                    found
                }
            };
            if found.is_some() {
                return found;
            }
        }
        None
    }
}

/// Icon theme configured in KDE (`kdeglobals [Icons] Theme`), defaulting to Breeze.
pub fn current_theme(home: &Path) -> String {
    let text = std::fs::read_to_string(home.join(".config/kdeglobals")).unwrap_or_default();
    let mut in_icons = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_icons = line == "[Icons]";
        } else if in_icons
            && let Some(v) = line.strip_prefix("Theme=") {
                return v.trim().to_string();
            }
    }
    "breeze".to_string()
}

/// Parses `~/.config/user-dirs.dirs` (XDG_PICTURES_DIR="$HOME/Images" …).
fn user_dirs(home: &Path) -> Vec<(PathBuf, &'static str)> {
    let text = std::fs::read_to_string(home.join(".config/user-dirs.dirs")).unwrap_or_default();
    parse_user_dirs(&text, home)
}

fn parse_user_dirs(text: &str, home: &Path) -> Vec<(PathBuf, &'static str)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else { continue };
        let icon = match key.trim() {
            "XDG_DESKTOP_DIR" => "user-desktop",
            "XDG_DOCUMENTS_DIR" => "folder-documents",
            "XDG_DOWNLOAD_DIR" => "folder-download",
            "XDG_MUSIC_DIR" => "folder-music",
            "XDG_PICTURES_DIR" => "folder-pictures",
            "XDG_VIDEOS_DIR" => "folder-videos",
            "XDG_TEMPLATES_DIR" => "folder-templates",
            "XDG_PUBLICSHARE_DIR" => "folder-publicshare",
            _ => continue,
        };
        let value = value.trim().trim_matches('"');
        let path = match value.strip_prefix("$HOME") {
            Some(rest) => home.join(rest.trim_start_matches('/')),
            None => PathBuf::from(value),
        };
        if path != home {
            out.push((path, icon));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_user_dirs() {
        let home = Path::new("/home/u");
        let dirs = parse_user_dirs("XDG_PICTURES_DIR=\"$HOME/Images\"\nXDG_DESKTOP_DIR=\"$HOME\"\n# comment\n", home);
        assert_eq!(dirs, vec![(PathBuf::from("/home/u/Images"), "folder-pictures")]);
    }

    fn entry(name: &str, is_dir: bool) -> Entry {
        Entry { name: name.into(), path: PathBuf::from("/tmp").join(name), is_dir, is_symlink: false, size: 0, modified: None, hidden: false }
    }

    #[test]
    fn icon_names_for_files_and_dirs() {
        let r = IconResolver::new(Path::new("/nonexistent-home"));
        assert_eq!(r.icon_names(&entry("x", true)), ["folder"]);
        let names = r.icon_names(&entry("photo.jpg", false));
        // With a shared-mime-info database present, the specific icon comes first.
        if std::path::Path::new("/usr/share/mime/globs2").exists() {
            assert_eq!(names[0], "image-jpeg");
        }
        assert_eq!(names.last().unwrap(), "unknown");
    }
}

#[cfg(test)]
mod system_probe {
    /// Manual check against the real system theme: `cargo test -p satchel-core -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn probe_real_theme() {
        let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
        let mut r = super::IconResolver::new(&home);
        println!("theme = {}", r.theme());
        for (name, dir) in [("Images", true), ("dev", true), ("a.jpg", false), ("main.rs", false), ("x.pdf", false), ("noext", false)] {
            let e = crate::Entry::from_path(&home.join(name)).unwrap_or(crate::Entry {
                name: name.into(), path: home.join(name), is_dir: dir, is_symlink: false, size: 0, modified: None, hidden: false,
            });
            println!("{name:10} 16→ {:?}\n{:10} 64→ {:?}", r.resolve(&e, 16), "", r.resolve(&e, 64));
        }
    }
}
