//! Copy/move engine (spec 02 §12): never overwrites implicitly, asks the handler on every
//! conflict or error, supports "apply to all", progress, cancellation, and records undo.
//! Runs on a worker thread; the handler may block waiting for the user's answer.

use super::fsutil::{self, copy_file_atomic, copy_symlink, create_dir, is_same_or_inside, remove_any, rename_noreplace, sync_dir, tree_size};
use super::names;
use super::trash::{self, TrashConfig, TrashError, Trashed};
use super::undo::UndoRecord;
use std::collections::HashMap;
use std::fs::{self, Metadata};
use std::io;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferKind {
    Copy,
    Move,
}

#[derive(Debug, Clone)]
pub struct Conflict {
    pub source: PathBuf,
    pub target: PathBuf,
    pub source_is_dir: bool,
    pub target_is_dir: bool,
    pub source_size: u64,
    pub target_size: u64,
    pub source_modified: Option<SystemTime>,
    pub target_modified: Option<SystemTime>,
    /// Copying an item onto itself (same folder): only rename/skip make sense.
    pub same_item: bool,
}

impl Conflict {
    /// File over file only (folder ↔ file can never be overwritten).
    pub fn can_overwrite(&self) -> bool {
        !self.same_item && !self.source_is_dir && !self.target_is_dir
    }
    /// Folder into folder: "Fusionner".
    pub fn can_merge(&self) -> bool {
        !self.same_item && self.source_is_dir && self.target_is_dir
    }
    pub fn source_is_newer(&self) -> bool {
        matches!((self.source_modified, self.target_modified), (Some(s), Some(t)) if s > t)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    Overwrite,
    OverwriteIfOlder,
    Skip,
    Rename(String),
    AutoRename,
    Merge,
    Cancel,
}

#[derive(Debug, Clone)]
pub struct Decision {
    pub resolution: Resolution,
    pub apply_to_all: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorAction {
    Skip,
    SkipAll,
    Retry,
    Cancel,
}

pub trait TransferHandler {
    fn conflict(&mut self, c: &Conflict) -> Decision;
    fn error(&mut self, path: &Path, err: &io::Error) -> ErrorAction;
    /// The overwritten target cannot go to the trash (other filesystem without trash): may it be
    /// lost for good? Default: no (the item is skipped).
    fn confirm_overwrite_without_trash(&mut self, _target: &Path) -> bool {
        false
    }
    fn progress(&mut self, _done: u64, _total: u64, _current: &Path) {}
}

#[derive(Debug, Default)]
pub struct TransferReport {
    /// Top-level results at the destination (to select them, spec 02 §10 / §17 #28).
    pub results: Vec<PathBuf>,
    pub skipped: usize,
    pub cancelled: bool,
    pub undo: Option<UndoRecord>,
}

/// One item to transfer: source, destination folder, name to give it there.
#[derive(Debug, Clone)]
pub struct Item {
    pub source: PathBuf,
    pub dest_dir: PathBuf,
    pub name: String,
}

/// Drops items whose ancestor is also in the list (spec 02 §17 #14).
pub fn simplify(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths.iter().filter(|p| !paths.iter().any(|q| q != *p && p.starts_with(q))).cloned().collect()
}

/// Copies or moves `sources` into `dest_dir`, keeping their names (conflicts resolved by the handler).
pub fn transfer(kind: TransferKind, sources: &[PathBuf], dest_dir: &Path, handler: &mut dyn TransferHandler, cancel: &AtomicBool, trash_cfg: &TrashConfig) -> TransferReport {
    let items = simplify(sources)
        .into_iter()
        .filter_map(|s| {
            let name = s.file_name()?.to_string_lossy().into_owned();
            Some(Item { source: s, dest_dir: dest_dir.to_path_buf(), name })
        })
        .collect();
    transfer_items(kind, items, handler, cancel, trash_cfg)
}

/// Copies `paths` next to themselves as "nom copie.ext" (Ctrl+D). Names are always free.
pub fn duplicate(paths: &[PathBuf], handler: &mut dyn TransferHandler, cancel: &AtomicBool, trash_cfg: &TrashConfig) -> TransferReport {
    let items = simplify(paths)
        .into_iter()
        .filter_map(|s| {
            let dir = s.parent()?.to_path_buf();
            let name = names::duplicate_name(&dir, &s.file_name()?.to_string_lossy());
            Some(Item { source: s, dest_dir: dir, name })
        })
        .collect();
    transfer_items(TransferKind::Copy, items, handler, cancel, trash_cfg)
}

pub fn transfer_items(kind: TransferKind, items: Vec<Item>, handler: &mut dyn TransferHandler, cancel: &AtomicBool, trash_cfg: &TrashConfig) -> TransferReport {
    let total = items.iter().map(|i| tree_size(&i.source)).sum();
    let mut e = Engine {
        handler,
        cancel,
        trash_cfg,
        policies: HashMap::new(),
        skip_all_errors: false,
        done: 0,
        total,
        created: Vec::new(),
        moved: Vec::new(),
        created_dirs: Vec::new(),
        removed_dirs: Vec::new(),
        replaced: Vec::new(),
        skipped: 0,
    };
    let mut report = TransferReport::default();
    for item in items {
        match e.item(&item.source, &item.dest_dir, item.name, kind) {
            Step::Done(target) => report.results.push(target),
            Step::Skipped => e.skipped += 1,
            Step::Cancelled => {
                report.cancelled = true;
                break;
            }
        }
    }
    report.skipped = e.skipped;
    let has_changes = !(e.created.is_empty() && e.moved.is_empty() && e.created_dirs.is_empty() && e.replaced.is_empty());
    if has_changes {
        report.undo = Some(match kind {
            TransferKind::Copy => UndoRecord::Copy { created: e.created, replaced: e.replaced },
            TransferKind::Move => UndoRecord::Move { moved: e.moved, created_dirs: e.created_dirs, removed_dirs: e.removed_dirs, replaced: e.replaced },
        });
    }
    report
}

enum Step {
    Done(PathBuf),
    Skipped,
    Cancelled,
}

enum Stop {
    Skipped,
    Cancelled,
}

struct Engine<'a> {
    handler: &'a mut dyn TransferHandler,
    cancel: &'a AtomicBool,
    trash_cfg: &'a TrashConfig,
    /// "Appliquer à tous" memory, per (source is dir, target is dir) category.
    policies: HashMap<(bool, bool), Resolution>,
    skip_all_errors: bool,
    done: u64,
    total: u64,
    created: Vec<(PathBuf, Option<SystemTime>)>,
    moved: Vec<(PathBuf, PathBuf)>,
    created_dirs: Vec<PathBuf>,
    removed_dirs: Vec<PathBuf>,
    replaced: Vec<Trashed>,
    skipped: usize,
}

fn is_xdev(e: &io::Error) -> bool {
    e.raw_os_error() == Some(rustix::io::Errno::XDEV.raw_os_error())
}

fn into_itself() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "impossible de placer un dossier dans lui-même")
}

impl Engine<'_> {
    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// Runs `op`, asking the handler what to do on error (retry / skip / skip all / cancel).
    fn attempt<T>(&mut self, path: &Path, mut op: impl FnMut(&mut Self) -> io::Result<T>) -> Result<T, Stop> {
        loop {
            if self.cancelled() {
                return Err(Stop::Cancelled);
            }
            match op(self) {
                Ok(v) => return Ok(v),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => return Err(Stop::Cancelled),
                Err(_) if self.skip_all_errors => return Err(Stop::Skipped),
                Err(e) => match self.handler.error(path, &e) {
                    ErrorAction::Retry => continue,
                    ErrorAction::Skip => return Err(Stop::Skipped),
                    ErrorAction::SkipAll => {
                        self.skip_all_errors = true;
                        return Err(Stop::Skipped);
                    }
                    ErrorAction::Cancel => return Err(Stop::Cancelled),
                },
            }
        }
    }

    fn step<T>(r: Result<T, Stop>) -> Result<T, Step> {
        r.map_err(|s| match s {
            Stop::Skipped => Step::Skipped,
            Stop::Cancelled => Step::Cancelled,
        })
    }

    /// Transfers `src` into `dest_dir` under `name`, resolving conflicts.
    fn item(&mut self, src: &Path, dest_dir: &Path, name: String, kind: TransferKind) -> Step {
        match self.item_inner(src, dest_dir, name, kind) {
            Ok(target) => Step::Done(target),
            Err(step) => step,
        }
    }

    fn item_inner(&mut self, src: &Path, dest_dir: &Path, mut name: String, kind: TransferKind) -> Result<PathBuf, Step> {
        let src_meta = Self::step(self.attempt(src, |_| fs::symlink_metadata(src)))?;
        if src_meta.is_dir() && is_same_or_inside(dest_dir, src) && !(kind == TransferKind::Copy && dest_dir == src.parent().unwrap_or(src)) {
            Self::step(self.attempt(src, |_| Err::<(), _>(into_itself())))?;
        }
        loop {
            let target = dest_dir.join(&name);
            let target_meta = match fs::symlink_metadata(&target) {
                Ok(m) => m,
                Err(e) if e.kind() == io::ErrorKind::NotFound => return self.perform(src, &src_meta, &target, kind),
                Err(e) => {
                    let err = e;
                    Self::step(self.attempt(&target, |_| Err::<(), _>(io::Error::new(err.kind(), err.to_string()))))?;
                    continue;
                }
            };
            let same_item = target_meta.dev() == src_meta.dev() && target_meta.ino() == src_meta.ino();
            if same_item && kind == TransferKind::Move {
                return Err(Step::Skipped); // moving onto itself: nothing to do
            }
            let conflict = Conflict {
                source: src.to_path_buf(),
                target: target.clone(),
                source_is_dir: src_meta.is_dir(),
                target_is_dir: target_meta.is_dir(),
                source_size: src_meta.len(),
                target_size: target_meta.len(),
                source_modified: src_meta.modified().ok(),
                target_modified: target_meta.modified().ok(),
                same_item,
            };
            let category = (conflict.source_is_dir, conflict.target_is_dir);
            let resolution = match self.policies.get(&category) {
                Some(r) if !same_item => r.clone(),
                _ => {
                    let d = self.handler.conflict(&conflict);
                    if d.apply_to_all && !same_item {
                        let remembered = match &d.resolution {
                            Resolution::Rename(_) => Resolution::AutoRename,
                            r => r.clone(),
                        };
                        self.policies.insert(category, remembered);
                    }
                    d.resolution
                }
            };
            match resolution {
                Resolution::Cancel => return Err(Step::Cancelled),
                Resolution::Skip => return Err(Step::Skipped),
                Resolution::Rename(new) => match names::sanitize(&new) {
                    Ok(n) => name = n,
                    Err(_) => continue, // ask again
                },
                Resolution::AutoRename => name = names::suggest_name(dest_dir, &name),
                Resolution::OverwriteIfOlder if conflict.can_overwrite() => {
                    if conflict.source_is_newer() {
                        return self.overwrite(src, &src_meta, &target, kind);
                    }
                    return Err(Step::Skipped);
                }
                Resolution::Overwrite if conflict.can_overwrite() => return self.overwrite(src, &src_meta, &target, kind),
                Resolution::Merge if conflict.can_merge() => return self.merge(src, &target, kind),
                _ => return Err(Step::Skipped), // invalid choice for this conflict: be safe
            }
        }
    }

    /// The old target goes to the trash first (recoverable, undoable), then the normal transfer.
    fn overwrite(&mut self, src: &Path, src_meta: &Metadata, target: &Path, kind: TransferKind) -> Result<PathBuf, Step> {
        match trash::trash(target, self.trash_cfg) {
            Ok(t) => self.replaced.push(t),
            Err(TrashError::Unavailable) => {
                if !self.handler.confirm_overwrite_without_trash(target) {
                    return Err(Step::Skipped);
                }
                Self::step(self.attempt(target, |_| remove_any(target)))?;
            }
            Err(TrashError::Io(e)) => {
                let err = e;
                Self::step(self.attempt(target, |_| Err::<(), _>(io::Error::new(err.kind(), err.to_string()))))?;
            }
        }
        self.perform(src, src_meta, target, kind)
    }

    /// Folder into existing folder: transfer children one by one (their conflicts are asked).
    fn merge(&mut self, src: &Path, target: &Path, kind: TransferKind) -> Result<PathBuf, Step> {
        let children = Self::step(self.attempt(src, |_| sorted_children(src)))?;
        for child in children {
            let name = child.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            match self.item(&child, target, name, kind) {
                Step::Done(_) => {}
                Step::Skipped => self.skipped += 1,
                Step::Cancelled => return Err(Step::Cancelled),
            }
        }
        if kind == TransferKind::Move && fs::remove_dir(src).is_ok() {
            self.removed_dirs.push(src.to_path_buf());
        }
        Ok(target.to_path_buf())
    }

    /// `target` does not exist: copy or move `src` there.
    fn perform(&mut self, src: &Path, meta: &Metadata, target: &Path, kind: TransferKind) -> Result<PathBuf, Step> {
        if kind == TransferKind::Move {
            match rename_noreplace(src, target) {
                Ok(()) => {
                    self.done += tree_size(target);
                    self.handler.progress(self.done, self.total, target);
                    self.moved.push((src.to_path_buf(), target.to_path_buf()));
                    return Ok(target.to_path_buf());
                }
                Err(e) if is_xdev(&e) => return self.move_across(src, meta, target),
                Err(e) => {
                    let err = e;
                    Self::step(self.attempt(src, |_| Err::<(), _>(io::Error::new(err.kind(), err.to_string()))))?;
                    return Err(Step::Skipped);
                }
            }
        }
        self.copy(src, meta, target)
    }

    fn copy(&mut self, src: &Path, meta: &Metadata, target: &Path) -> Result<PathBuf, Step> {
        let ft = meta.file_type();
        if ft.is_symlink() {
            Self::step(self.attempt(src, |_| copy_symlink(src, target)))?;
            self.created.push((target.to_path_buf(), None));
        } else if ft.is_file() {
            Self::step(self.attempt(src, |e| {
                let cancel = e.cancel;
                let (mut done, total) = (e.done, e.total);
                let handler = &mut *e.handler;
                let r = copy_file_atomic(src, target, cancel, &mut |n| {
                    done += n;
                    handler.progress(done, total, target);
                });
                if r.is_ok() {
                    e.done = done;
                }
                r
            }))?;
            let mtime = fs::symlink_metadata(target).ok().and_then(|m| m.modified().ok());
            self.created.push((target.to_path_buf(), mtime));
        } else if ft.is_dir() {
            Self::step(self.attempt(target, |_| create_dir(target)))?;
            self.created.push((target.to_path_buf(), None));
            let children = Self::step(self.attempt(src, |_| sorted_children(src)))?;
            for child in children {
                let name = child.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                match self.item(&child, target, name, TransferKind::Copy) {
                    Step::Done(_) => {}
                    Step::Skipped => self.skipped += 1,
                    Step::Cancelled => return Err(Step::Cancelled),
                }
            }
            let _ = fs::set_permissions(target, fs::Permissions::from_mode(meta.permissions().mode()));
        } else {
            Self::step(self.attempt(src, |_| Err::<(), _>(io::Error::new(io::ErrorKind::Unsupported, "type de fichier spécial non copié"))))?;
        }
        Ok(target.to_path_buf())
    }

    /// Cross-filesystem move: copy, make it durable, then delete the source — per file, so an
    /// interruption never loses data (spec 02 §12, §17 #9).
    fn move_across(&mut self, src: &Path, meta: &Metadata, target: &Path) -> Result<PathBuf, Step> {
        let ft = meta.file_type();
        if ft.is_dir() {
            Self::step(self.attempt(target, |_| create_dir(target)))?;
            self.created_dirs.push(target.to_path_buf());
            let children = Self::step(self.attempt(src, |_| sorted_children(src)))?;
            for child in children {
                let name = child.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                match self.item(&child, target, name, TransferKind::Move) {
                    Step::Done(_) => {}
                    Step::Skipped => self.skipped += 1,
                    Step::Cancelled => return Err(Step::Cancelled),
                }
            }
            let _ = fs::set_permissions(target, fs::Permissions::from_mode(meta.permissions().mode()));
            if fs::remove_dir(src).is_ok() {
                self.removed_dirs.push(src.to_path_buf());
            }
            return Ok(target.to_path_buf());
        }
        // File or symlink: copy (atomic), fsync the destination folder, then remove the source.
        let created_before = self.created.len();
        self.copy(src, meta, target)?;
        self.created.truncate(created_before); // it is a move, not a copy, for undo purposes
        if let Some(parent) = target.parent() {
            let _ = sync_dir(parent);
        }
        Self::step(self.attempt(src, |_| fs::remove_file(src)))?;
        self.moved.push((src.to_path_buf(), target.to_path_buf()));
        Ok(target.to_path_buf())
    }
}

fn sorted_children(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)?.filter_map(|e| e.ok().map(|e| e.path())).collect();
    v.sort();
    Ok(v)
}

// Keep the part suffix visible to callers that list directories (hide in-flight files).
pub use fsutil::PART_SUFFIX;
