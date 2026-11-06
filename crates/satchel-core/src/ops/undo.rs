//! Undo records (spec 02 §13). Each record re-checks the disk before acting: a copied file that
//! was modified since the copy is not deleted unless the user confirmed (`force_modified`).

use super::fsutil::{rename_noreplace, same_device, copy_file_atomic, copy_symlink, remove_any};
use super::trash::{self, Trashed};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub enum UndoRecord {
    /// Copies created (files, symlinks, dirs in creation order) + overwritten targets sent to trash.
    Copy { created: Vec<(PathBuf, Option<SystemTime>)>, replaced: Vec<Trashed> },
    /// (from, to) leaf moves; dirs created at the destination / removed at the source (cross-device
    /// or merge moves, in order); overwritten targets sent to trash.
    Move { moved: Vec<(PathBuf, PathBuf)>, created_dirs: Vec<PathBuf>, removed_dirs: Vec<PathBuf>, replaced: Vec<Trashed> },
    Rename { from: PathBuf, to: PathBuf },
    Trash { items: Vec<Trashed> },
    Mkdir { path: PathBuf },
}

impl UndoRecord {
    /// Label for "Annuler : …" in the UI.
    pub fn label(&self) -> &'static str {
        match self {
            UndoRecord::Copy { .. } => "Copie",
            UndoRecord::Move { .. } => "Déplacement",
            UndoRecord::Rename { .. } => "Renommage",
            UndoRecord::Trash { .. } => "Mise à la corbeille",
            UndoRecord::Mkdir { .. } => "Nouveau dossier",
        }
    }

    /// Undoing a copy deletes files: the UI must confirm (spec 02 §13).
    pub fn deletes_files(&self) -> bool {
        matches!(self, UndoRecord::Copy { created, .. } if !created.is_empty())
    }

    /// Copied files modified since the copy (their changes would be lost by undo).
    pub fn modified_since(&self) -> Vec<PathBuf> {
        match self {
            UndoRecord::Copy { created, .. } => created
                .iter()
                .filter(|(p, m)| m.is_some() && fs::symlink_metadata(p).ok().and_then(|x| x.modified().ok()) != *m && p.exists())
                .map(|(p, _)| p.clone())
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Applies the undo. Returns per-item errors (the rest is still undone).
    pub fn undo(&self, force_modified: bool) -> Vec<(PathBuf, io::Error)> {
        let mut errors = Vec::new();
        match self {
            UndoRecord::Copy { created, replaced } => {
                for (path, mtime) in created.iter().rev() {
                    let Ok(meta) = fs::symlink_metadata(path) else { continue }; // already gone
                    let r = if meta.is_dir() {
                        fs::remove_dir(path) // only if empty: never deletes things added since
                    } else if mtime.is_some() && meta.modified().ok() != *mtime && !force_modified {
                        Err(io::Error::other("modifié depuis la copie : conservé"))
                    } else {
                        fs::remove_file(path)
                    };
                    if let Err(e) = r {
                        errors.push((path.clone(), e));
                    }
                }
                restore_replaced(replaced, &mut errors);
            }
            UndoRecord::Move { moved, created_dirs, removed_dirs, replaced } => {
                // Recreate source dirs removed after emptying (they were recorded child-first).
                for d in removed_dirs.iter().rev() {
                    if let Err(e) = fs::create_dir_all(d) {
                        errors.push((d.clone(), e));
                    }
                }
                for (from, to) in moved.iter().rev() {
                    if let Err(e) = move_back(to, from) {
                        errors.push((to.clone(), e));
                    }
                }
                for d in created_dirs.iter().rev() {
                    let _ = fs::remove_dir(d); // only if empty
                }
                restore_replaced(replaced, &mut errors);
            }
            UndoRecord::Rename { from, to } => {
                if let Err(e) = rename_noreplace(to, from) {
                    errors.push((to.clone(), e));
                }
            }
            UndoRecord::Trash { items } => {
                for t in items.iter().rev() {
                    if let Err(e) = trash::restore(t) {
                        errors.push((t.original.clone(), e));
                    }
                }
            }
            UndoRecord::Mkdir { path } => {
                if let Err(e) = fs::remove_dir(path) {
                    errors.push((path.clone(), e));
                }
            }
        }
        errors
    }
}

fn restore_replaced(replaced: &[Trashed], errors: &mut Vec<(PathBuf, io::Error)>) {
    for t in replaced.iter().rev() {
        if let Err(e) = trash::restore(t) {
            errors.push((t.original.clone(), e));
        }
    }
}

/// Moves a single item back, without replacing anything; cross-device via copy then delete.
fn move_back(current: &Path, original: &Path) -> io::Result<()> {
    match rename_noreplace(current, original) {
        Ok(()) => Ok(()),
        Err(e) if e.raw_os_error() == Some(rustix::io::Errno::XDEV.raw_os_error()) || !same_device_parent(current, original) => {
            let meta = fs::symlink_metadata(current)?;
            if meta.file_type().is_symlink() {
                copy_symlink(current, original)?;
            } else if meta.is_file() {
                copy_file_atomic(current, original, &AtomicBool::new(false), &mut |_| {})?;
            } else {
                return Err(e);
            }
            remove_any(current)
        }
        Err(e) => Err(e),
    }
}

fn same_device_parent(a: &Path, b: &Path) -> bool {
    match (a.parent(), b.parent()) {
        (Some(x), Some(y)) => same_device(x, y),
        _ => true,
    }
}

/// Application-wide undo stack (spec 02 §13: global, 50 entries, not persisted).
#[derive(Default)]
pub struct UndoStack {
    entries: Vec<UndoRecord>,
}

impl UndoStack {
    const MAX: usize = 50;

    pub fn push(&mut self, rec: UndoRecord) {
        self.entries.push(rec);
        if self.entries.len() > Self::MAX {
            self.entries.remove(0);
        }
    }

    pub fn peek(&self) -> Option<&UndoRecord> {
        self.entries.last()
    }

    pub fn pop(&mut self) -> Option<UndoRecord> {
        self.entries.pop()
    }
}
