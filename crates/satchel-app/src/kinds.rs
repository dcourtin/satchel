//! File kind heuristics by extension (prototype; real MIME detection via shared-mime-info later).
use satchel_core::Entry;

pub fn extension(name: &str) -> &str {
    match name.rfind('.') {
        Some(0) | None => "",
        Some(i) => &name[i + 1..],
    }
}

fn ext_lower(e: &Entry) -> String {
    extension(&e.name).to_ascii_lowercase()
}

pub fn is_image(e: &Entry) -> bool {
    matches!(ext_lower(e).as_str(), "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp")
}

pub fn kind(e: &Entry) -> &'static str {
    if e.is_dir {
        return "folder";
    }
    match ext_lower(e).as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "svg" | "tif" | "tiff" | "heic" | "avif" => "image",
        "mp3" | "flac" | "ogg" | "wav" | "m4a" | "opus" => "audio",
        "mp4" | "mkv" | "webm" | "avi" | "mov" => "video",
        "zip" | "tar" | "gz" | "xz" | "zst" | "7z" | "rar" | "bz2" => "archive",
        "rs" | "py" | "js" | "ts" | "c" | "h" | "cpp" | "php" | "html" | "css" | "json" | "toml" | "yaml" | "yml"
        | "sh" | "md" | "slint" | "go" | "java" | "kt" | "xml" | "kdl" | "conf" => "code",
        _ => "doc",
    }
}

pub fn kind_text(e: &Entry) -> String {
    let ext = ext_lower(e);
    let known = match ext.as_str() {
        "jpg" | "jpeg" => "Image JPEG",
        "png" => "Image PNG",
        "gif" => "Image GIF",
        "webp" => "Image WebP",
        "svg" => "Image SVG",
        "pdf" => "Document PDF",
        "txt" => "Texte brut",
        "md" => "Document Markdown",
        "zip" => "Archive ZIP",
        "mp3" => "Audio MP3",
        "flac" => "Audio FLAC",
        "mp4" => "Vidéo MP4",
        "mkv" => "Vidéo Matroska",
        "html" => "Page HTML",
        "php" => "Script PHP",
        "rs" => "Source Rust",
        _ => "",
    };
    if !known.is_empty() {
        known.to_string()
    } else if ext.is_empty() {
        "Fichier".to_string()
    } else {
        format!("Fichier {}", ext.to_uppercase())
    }
}

/// Maps freedesktop/KDE place icon names to our glyph set.
pub fn place_icon(name: &str) -> &'static str {
    match name {
        "user-home" => "home",
        "user-desktop" => "desktop",
        "folder-download" | "folder-downloads" => "download",
        "folder-documents" => "doc",
        "folder-pictures" | "folder-images" => "image",
        "folder-music" => "music",
        "folder-videos" => "video",
        "user-trash" => "trash",
        "drive-harddisk" | "drive-removable-media" => "drive",
        _ => "folder",
    }
}

/// Lowercase + accent fold for type-ahead matching.
pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            c => c,
        })
        .collect()
}
