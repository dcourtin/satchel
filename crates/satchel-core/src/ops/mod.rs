//! File operations (Jalon 2). Everything that writes to disk lives here and is tested.
//! Rules: CLAUDE.md « Règles d'or » and docs/spec/02-vue-et-operations.md §8–13, §17.

pub mod fsutil;
pub mod job;
pub mod names;
pub mod trash;
pub mod undo;

#[cfg(test)]
mod tests;

pub use job::{Conflict, Decision, ErrorAction, Resolution, TransferHandler, TransferKind, TransferReport};
pub use trash::{TrashConfig, TrashError, Trashed};
pub use undo::{UndoRecord, UndoStack};

use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum OpError {
    InvalidName(names::NameError),
    /// The target name is taken (the UI offers a conflict choice or another name).
    Exists(PathBuf),
    Io(io::Error),
}

impl std::fmt::Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpError::InvalidName(e) => write!(f, "nom invalide : {e}"),
            OpError::Exists(p) => write!(f, "« {} » existe déjà", p.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()),
            OpError::Io(e) => write!(f, "{}", describe_io(e)),
        }
    }
}

impl From<io::Error> for OpError {
    fn from(e: io::Error) -> Self {
        OpError::Io(e)
    }
}

/// French, user-facing description of an I/O error.
pub fn describe_io(e: &io::Error) -> String {
    use io::ErrorKind::*;
    match e.kind() {
        PermissionDenied => "permission refusée".into(),
        NotFound => "introuvable (déplacé ou supprimé entre-temps ?)".into(),
        AlreadyExists => "un élément porte déjà ce nom".into(),
        StorageFull => "plus d'espace disponible sur le disque".into(),
        ReadOnlyFilesystem => "le disque est en lecture seule".into(),
        Interrupted => "opération annulée".into(),
        _ => e.to_string(),
    }
}

/// Renames `path` to `new_name` in the same folder, never replacing an existing item (spec 02 §9).
/// Returns Ok(None) when the name is unchanged (ignored silently, like Dolphin).
pub fn rename(path: &Path, new_name: &str) -> Result<Option<(PathBuf, UndoRecord)>, OpError> {
    let name = names::sanitize(new_name).map_err(OpError::InvalidName)?;
    let parent = path.parent().unwrap_or(Path::new("/"));
    let target = parent.join(&name);
    if target == path {
        return Ok(None);
    }
    match fsutil::rename_noreplace(path, &target) {
        Ok(()) => Ok(Some((target.clone(), UndoRecord::Rename { from: path.to_path_buf(), to: target }))),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Err(OpError::Exists(target)),
        Err(e) => Err(OpError::Io(e)),
    }
}

/// Creates `parent/name` (a `/` in the name becomes `∕`, no implicit hierarchy).
pub fn mkdir(parent: &Path, name: &str) -> Result<(PathBuf, UndoRecord), OpError> {
    let name = names::sanitize(name).map_err(OpError::InvalidName)?;
    let path = parent.join(&name);
    match fsutil::create_dir(&path) {
        Ok(()) => Ok((path.clone(), UndoRecord::Mkdir { path })),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Err(OpError::Exists(path)),
        Err(e) => Err(OpError::Io(e)),
    }
}

/// Default name proposed for a new folder (« Nouveau dossier », then « Nouveau dossier (1) »…).
pub fn new_folder_name(parent: &Path) -> String {
    let base = "Nouveau dossier";
    if std::fs::symlink_metadata(parent.join(base)).is_err() { base.to_string() } else { names::suggest_name(parent, base) }
}

/// Result of moving several items to the trash.
#[derive(Debug, Default)]
pub struct TrashReport {
    pub trashed: Vec<Trashed>,
    /// Items with no trash on their filesystem: the UI asks before deleting them permanently.
    pub unavailable: Vec<PathBuf>,
    pub errors: Vec<(PathBuf, io::Error)>,
}

impl TrashReport {
    pub fn undo(&self) -> Option<UndoRecord> {
        (!self.trashed.is_empty()).then(|| UndoRecord::Trash { items: self.trashed.clone() })
    }
}

pub fn trash_items(paths: &[PathBuf], cfg: &TrashConfig) -> TrashReport {
    let mut r = TrashReport::default();
    for p in job::simplify(paths) {
        match trash::trash(&p, cfg) {
            Ok(t) => r.trashed.push(t),
            Err(TrashError::Unavailable) => r.unavailable.push(p),
            Err(TrashError::Io(e)) => r.errors.push((p, e)),
        }
    }
    r
}

/// Permanent deletion (not undoable — the UI must have confirmed, spec 02 §17 #1).
/// Symlinks are removed, never followed.
pub fn delete_permanently(paths: &[PathBuf]) -> Vec<(PathBuf, io::Error)> {
    job::simplify(paths)
        .into_iter()
        .filter_map(|p| fsutil::remove_any(&p).err().map(|e| (p, e)))
        .collect()
}
