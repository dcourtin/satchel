//! File-name rules for operations (spec 02 §9, §10, §12).

use std::path::Path;

/// Extensions treated as a single unit when splitting a name ("archive.tar.gz" → "archive" + ".tar.gz").
const DOUBLE_EXTENSIONS: &[&str] = &[".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst", ".tar.lz", ".tar.lzma", ".tar.Z"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameError {
    Empty,
    Reserved, // "." or ".."
    Nul,
}

impl std::fmt::Display for NameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NameError::Empty => write!(f, "le nom ne peut pas être vide"),
            NameError::Reserved => write!(f, "« . » et « .. » sont des noms réservés"),
            NameError::Nul => write!(f, "le nom contient un caractère interdit"),
        }
    }
}

/// Validates a user-typed name. A `/` is replaced by `∕` (U+2215), like KIO's `encodeFileName`,
/// so a name can never silently become a path (spec 02 §17 #17).
pub fn sanitize(name: &str) -> Result<String, NameError> {
    if name.contains('\0') {
        return Err(NameError::Nul);
    }
    let cleaned = name.replace('/', "\u{2215}");
    match cleaned.as_str() {
        "" => Err(NameError::Empty),
        "." | ".." => Err(NameError::Reserved),
        _ => Ok(cleaned),
    }
}

/// Splits "photo.jpg" → ("photo", ".jpg"), "a.tar.gz" → ("a", ".tar.gz"), ".bashrc" → (".bashrc", "").
pub fn split_extension(name: &str) -> (&str, &str) {
    for ext in DOUBLE_EXTENSIONS {
        if name.len() > ext.len() && name.ends_with(ext) {
            let cut = name.len() - ext.len();
            return (&name[..cut], &name[cut..]);
        }
    }
    match name.rfind('.') {
        Some(0) | None => (name, ""),
        Some(i) if i == name.len() - 1 => (name, ""),
        Some(i) => (&name[..i], &name[i..]),
    }
}

/// Part of the name preselected in the rename editor: everything but the extension; the whole
/// name for folders (spec 01 §8 / 02 §9). Returned as a char range.
pub fn rename_selection(name: &str, is_dir: bool) -> std::ops::Range<usize> {
    let total = name.chars().count();
    if is_dir {
        return 0..total;
    }
    let (base, _) = split_extension(name);
    0..base.chars().count()
}

fn exists(dir: &Path, name: &str) -> bool {
    // symlink_metadata so that a broken symlink also counts as "taken".
    std::fs::symlink_metadata(dir.join(name)).is_ok()
}

/// Free name for a conflict: "nom (1).ext", "nom (2).ext"… An existing " (N)" suffix is incremented.
pub fn suggest_name(dir: &Path, name: &str) -> String {
    let (base, ext) = split_extension(name);
    let (stem, mut n) = match base.rfind(" (") {
        Some(i) if base.ends_with(')') => match base[i + 2..base.len() - 1].parse::<u32>() {
            Ok(k) => (&base[..i], k + 1),
            Err(_) => (base, 1),
        },
        _ => (base, 1),
    };
    loop {
        let candidate = format!("{stem} ({n}){ext}");
        if !exists(dir, &candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// Name for "Dupliquer ici" (Ctrl+D): "nom copie.ext", then "nom copie (1).ext"… Never an existing name.
pub fn duplicate_name(dir: &Path, name: &str) -> String {
    let (base, ext) = split_extension(name);
    let candidate = format!("{base} copie{ext}");
    if exists(dir, &candidate) { suggest_name(dir, &candidate) } else { candidate }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes() {
        assert_eq!(sanitize("a/b"), Ok("a\u{2215}b".into()));
        assert_eq!(sanitize(""), Err(NameError::Empty));
        assert_eq!(sanitize(".."), Err(NameError::Reserved));
        assert_eq!(sanitize("a\0b"), Err(NameError::Nul));
        assert_eq!(sanitize(" espace "), Ok(" espace ".into()));
    }

    #[test]
    fn splits_extensions() {
        assert_eq!(split_extension("photo.jpg"), ("photo", ".jpg"));
        assert_eq!(split_extension("archive.tar.gz"), ("archive", ".tar.gz"));
        assert_eq!(split_extension(".bashrc"), (".bashrc", ""));
        assert_eq!(split_extension("noext"), ("noext", ""));
        assert_eq!(split_extension("trailing."), ("trailing.", ""));
        assert_eq!(rename_selection("archive.tar.gz", false), 0..7);
        assert_eq!(rename_selection("été.txt", false), 0..3);
        assert_eq!(rename_selection("dossier.d", true), 0..9);
    }

    #[test]
    fn suggests_free_names() {
        let tmp = tempfile::tempdir().unwrap();
        let d = tmp.path();
        assert_eq!(suggest_name(d, "a.txt"), "a (1).txt");
        std::fs::write(d.join("a (1).txt"), "").unwrap();
        assert_eq!(suggest_name(d, "a.txt"), "a (2).txt");
        assert_eq!(suggest_name(d, "a (1).txt"), "a (2).txt");
        assert_eq!(suggest_name(d, "x.tar.gz"), "x (1).tar.gz");
        assert_eq!(suggest_name(d, "photo (vacances).jpg"), "photo (vacances) (1).jpg");
    }

    #[test]
    fn duplicate_names() {
        let tmp = tempfile::tempdir().unwrap();
        let d = tmp.path();
        assert_eq!(duplicate_name(d, "rapport.pdf"), "rapport copie.pdf");
        std::fs::write(d.join("rapport copie.pdf"), "").unwrap();
        assert_eq!(duplicate_name(d, "rapport.pdf"), "rapport copie (1).pdf");
        assert_eq!(duplicate_name(d, "Dossier"), "Dossier copie");
    }

    #[cfg(unix)]
    #[test]
    fn broken_symlink_counts_as_taken() {
        let tmp = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink("/nonexistent", tmp.path().join("l")).unwrap();
        assert_eq!(suggest_name(tmp.path(), "l"), "l (1)");
    }
}
