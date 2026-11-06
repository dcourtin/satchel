//! Behaviour tests for transfers, conflicts, trash and undo (spec 02 §12, §13, §17).

use super::job::{self, Conflict, Decision, ErrorAction, Resolution, TransferHandler, TransferKind};
use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

/// Scripted handler: answers conflicts from a queue, records what was asked.
struct Script {
    answers: Vec<Decision>,
    asked: Vec<Conflict>,
    error_action: ErrorAction,
    errors: Vec<PathBuf>,
    allow_lossy_overwrite: bool,
}

impl Script {
    fn new(answers: Vec<(Resolution, bool)>) -> Self {
        Script {
            answers: answers.into_iter().rev().map(|(resolution, apply_to_all)| Decision { resolution, apply_to_all }).collect(),
            asked: Vec::new(),
            error_action: ErrorAction::Skip,
            errors: Vec::new(),
            allow_lossy_overwrite: false,
        }
    }
}

impl TransferHandler for Script {
    fn conflict(&mut self, c: &Conflict) -> Decision {
        self.asked.push(c.clone());
        self.answers.pop().unwrap_or(Decision { resolution: Resolution::Cancel, apply_to_all: false })
    }
    fn error(&mut self, path: &Path, _err: &std::io::Error) -> ErrorAction {
        self.errors.push(path.to_path_buf());
        self.error_action
    }
    fn confirm_overwrite_without_trash(&mut self, _t: &Path) -> bool {
        self.allow_lossy_overwrite
    }
}

struct Fixture {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    cfg: TrashConfig,
}

fn fixture() -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("dst")).unwrap();
    let cfg = TrashConfig { home_trash: root.join("data/Trash") };
    Fixture { _tmp: tmp, root, cfg }
}

fn write(p: &Path, s: &str) {
    if let Some(d) = p.parent() {
        fs::create_dir_all(d).unwrap();
    }
    fs::write(p, s).unwrap();
}

fn read(p: &Path) -> String {
    fs::read_to_string(p).unwrap()
}

fn no_part_files(dir: &Path) -> bool {
    walk(dir).iter().all(|p| !p.to_string_lossy().ends_with(job::PART_SUFFIX))
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            out.push(p.clone());
            if fs::symlink_metadata(&p).map(|m| m.is_dir()).unwrap_or(false) {
                out.extend(walk(&p));
            }
        }
    }
    out
}

fn copy(f: &Fixture, srcs: &[&str], script: &mut Script) -> TransferReport {
    let s: Vec<PathBuf> = srcs.iter().map(|n| f.root.join("src").join(n)).collect();
    job::transfer(TransferKind::Copy, &s, &f.root.join("dst"), script, &AtomicBool::new(false), &f.cfg)
}

fn mv(f: &Fixture, srcs: &[&str], script: &mut Script) -> TransferReport {
    let s: Vec<PathBuf> = srcs.iter().map(|n| f.root.join("src").join(n)).collect();
    job::transfer(TransferKind::Move, &s, &f.root.join("dst"), script, &AtomicBool::new(false), &f.cfg)
}

#[test]
fn copies_tree_without_conflicts() {
    let f = fixture();
    write(&f.root.join("src/d/a.txt"), "A");
    write(&f.root.join("src/d/sub/b.txt"), "B");
    let mut s = Script::new(vec![]);
    let r = copy(&f, &["d"], &mut s);
    assert!(s.asked.is_empty() && !r.cancelled);
    assert_eq!(r.results, vec![f.root.join("dst/d")]);
    assert_eq!(read(&f.root.join("dst/d/sub/b.txt")), "B");
    assert_eq!(read(&f.root.join("src/d/a.txt")), "A", "source untouched");
    assert!(no_part_files(&f.root));
}

#[test]
fn conflict_skip_keeps_destination() {
    let f = fixture();
    write(&f.root.join("src/a.txt"), "new");
    write(&f.root.join("dst/a.txt"), "old");
    let mut s = Script::new(vec![(Resolution::Skip, false)]);
    let r = copy(&f, &["a.txt"], &mut s);
    assert_eq!(s.asked.len(), 1);
    assert!(s.asked[0].can_overwrite());
    assert_eq!(r.skipped, 1);
    assert_eq!(read(&f.root.join("dst/a.txt")), "old");
}

#[test]
fn overwrite_sends_old_to_trash_and_undo_restores_it() {
    let f = fixture();
    write(&f.root.join("src/a.txt"), "new");
    write(&f.root.join("dst/a.txt"), "old");
    let mut s = Script::new(vec![(Resolution::Overwrite, false)]);
    let r = copy(&f, &["a.txt"], &mut s);
    assert_eq!(read(&f.root.join("dst/a.txt")), "new");
    let trashed = walk(&f.cfg.home_trash.join("files"));
    assert_eq!(trashed.len(), 1);
    assert_eq!(read(&trashed[0]), "old", "overwritten file is recoverable");

    let undo = r.undo.unwrap();
    assert!(undo.undo(false).is_empty());
    assert_eq!(read(&f.root.join("dst/a.txt")), "old", "undo brings the old file back");
}

#[test]
fn autorename_and_apply_to_all() {
    let f = fixture();
    for n in ["a.txt", "b.txt"] {
        write(&f.root.join("src").join(n), "new");
        write(&f.root.join("dst").join(n), "old");
    }
    let mut s = Script::new(vec![(Resolution::AutoRename, true)]);
    let r = copy(&f, &["a.txt", "b.txt"], &mut s);
    assert_eq!(s.asked.len(), 1, "second conflict answered by « appliquer à tous »");
    assert_eq!(read(&f.root.join("dst/a (1).txt")), "new");
    assert_eq!(read(&f.root.join("dst/b (1).txt")), "new");
    assert_eq!(read(&f.root.join("dst/a.txt")), "old");
    assert_eq!(r.results.len(), 2);
}

#[test]
fn rename_choice_then_reask_if_taken() {
    let f = fixture();
    write(&f.root.join("src/a.txt"), "new");
    write(&f.root.join("dst/a.txt"), "old");
    write(&f.root.join("dst/taken.txt"), "x");
    let mut s = Script::new(vec![(Resolution::Rename("taken.txt".into()), false), (Resolution::Rename("free.txt".into()), false)]);
    copy(&f, &["a.txt"], &mut s);
    assert_eq!(s.asked.len(), 2);
    assert_eq!(read(&f.root.join("dst/free.txt")), "new");
    assert_eq!(read(&f.root.join("dst/taken.txt")), "x");
}

#[test]
fn folder_over_file_cannot_be_overwritten() {
    let f = fixture();
    write(&f.root.join("src/x/inner"), "i");
    write(&f.root.join("dst/x"), "file");
    let mut s = Script::new(vec![(Resolution::Overwrite, false)]);
    let r = copy(&f, &["x"], &mut s);
    assert!(!s.asked[0].can_overwrite() && !s.asked[0].can_merge());
    assert_eq!(r.skipped, 1);
    assert_eq!(read(&f.root.join("dst/x")), "file");
}

#[test]
fn merge_folders_asks_for_inner_conflicts() {
    let f = fixture();
    write(&f.root.join("src/d/only_src"), "s");
    write(&f.root.join("src/d/both"), "new");
    write(&f.root.join("dst/d/only_dst"), "d");
    write(&f.root.join("dst/d/both"), "old");
    let mut s = Script::new(vec![(Resolution::Merge, false), (Resolution::Skip, false)]);
    copy(&f, &["d"], &mut s);
    assert_eq!(s.asked.len(), 2);
    assert_eq!(read(&f.root.join("dst/d/only_src")), "s");
    assert_eq!(read(&f.root.join("dst/d/only_dst")), "d");
    assert_eq!(read(&f.root.join("dst/d/both")), "old");
}

#[test]
fn overwrite_if_older() {
    let f = fixture();
    write(&f.root.join("dst/a"), "old-but-newer");
    std::thread::sleep(std::time::Duration::from_millis(20));
    write(&f.root.join("src/a"), "newest");
    write(&f.root.join("src/b"), "older");
    std::thread::sleep(std::time::Duration::from_millis(20));
    write(&f.root.join("dst/b"), "newer");
    let mut s = Script::new(vec![(Resolution::OverwriteIfOlder, true)]);
    copy(&f, &["a", "b"], &mut s);
    assert_eq!(read(&f.root.join("dst/a")), "newest");
    assert_eq!(read(&f.root.join("dst/b")), "newer");
}

#[test]
fn copy_onto_itself_offers_rename_only() {
    let f = fixture();
    write(&f.root.join("src/a.txt"), "A");
    let mut s = Script::new(vec![(Resolution::AutoRename, false)]);
    let src = vec![f.root.join("src/a.txt")];
    job::transfer(TransferKind::Copy, &src, &f.root.join("src"), &mut s, &AtomicBool::new(false), &f.cfg);
    assert!(s.asked[0].same_item && !s.asked[0].can_overwrite());
    assert_eq!(read(&f.root.join("src/a (1).txt")), "A");
}

#[test]
fn move_same_fs_and_undo() {
    let f = fixture();
    write(&f.root.join("src/d/x"), "x");
    write(&f.root.join("src/f"), "f");
    let mut s = Script::new(vec![]);
    let r = mv(&f, &["d", "f"], &mut s);
    assert!(!f.root.join("src/d").exists() && !f.root.join("src/f").exists());
    assert_eq!(read(&f.root.join("dst/d/x")), "x");
    assert!(r.undo.as_ref().unwrap().undo(false).is_empty());
    assert_eq!(read(&f.root.join("src/d/x")), "x");
    assert_eq!(read(&f.root.join("src/f")), "f");
    assert!(!f.root.join("dst/d").exists());
}

#[test]
fn move_folder_into_itself_is_refused() {
    let f = fixture();
    write(&f.root.join("src/d/x"), "x");
    let mut s = Script::new(vec![]);
    let src = vec![f.root.join("src/d")];
    let r = job::transfer(TransferKind::Move, &src, &f.root.join("src/d"), &mut s, &AtomicBool::new(false), &f.cfg);
    assert_eq!(s.errors.len(), 1);
    assert!(r.results.is_empty());
    assert_eq!(read(&f.root.join("src/d/x")), "x");
}

#[test]
fn merge_move_removes_emptied_source_and_undo_recreates() {
    let f = fixture();
    write(&f.root.join("src/d/a"), "a");
    write(&f.root.join("dst/d/b"), "b");
    let mut s = Script::new(vec![(Resolution::Merge, false)]);
    let r = mv(&f, &["d"], &mut s);
    assert!(!f.root.join("src/d").exists());
    assert_eq!(read(&f.root.join("dst/d/a")), "a");
    assert!(r.undo.unwrap().undo(false).is_empty());
    assert_eq!(read(&f.root.join("src/d/a")), "a");
    assert_eq!(read(&f.root.join("dst/d/b")), "b");
}

#[test]
fn cancel_stops_and_leaves_no_partial_file() {
    let f = fixture();
    write(&f.root.join("src/a"), "a");
    let mut s = Script::new(vec![]);
    let r = job::transfer(TransferKind::Copy, &[f.root.join("src/a")], &f.root.join("dst"), &mut s, &AtomicBool::new(true), &f.cfg);
    assert!(r.cancelled);
    assert!(!f.root.join("dst/a").exists() && no_part_files(&f.root));
}

#[test]
fn symlinks_are_copied_not_followed() {
    let f = fixture();
    write(&f.root.join("outside/big"), "do not copy me");
    fs::create_dir_all(f.root.join("src/d")).unwrap();
    std::os::unix::fs::symlink(f.root.join("outside"), f.root.join("src/d/link")).unwrap();
    let mut s = Script::new(vec![]);
    copy(&f, &["d"], &mut s);
    let l = f.root.join("dst/d/link");
    assert!(fs::symlink_metadata(&l).unwrap().file_type().is_symlink());
    assert_eq!(fs::read_link(&l).unwrap(), f.root.join("outside"));
}

#[test]
fn undo_copy_keeps_files_modified_since() {
    let f = fixture();
    write(&f.root.join("src/a"), "a");
    write(&f.root.join("src/b"), "b");
    let r = copy(&f, &["a", "b"], &mut Script::new(vec![]));
    std::thread::sleep(std::time::Duration::from_millis(20));
    write(&f.root.join("dst/b"), "edited after copy");
    let undo = r.undo.unwrap();
    assert!(undo.deletes_files());
    assert_eq!(undo.modified_since(), vec![f.root.join("dst/b")]);
    let errs = undo.undo(false);
    assert_eq!(errs.len(), 1);
    assert!(!f.root.join("dst/a").exists());
    assert_eq!(read(&f.root.join("dst/b")), "edited after copy");
}

#[test]
fn duplicate_names_and_never_overwrites() {
    let f = fixture();
    write(&f.root.join("src/r.pdf"), "1");
    write(&f.root.join("src/r copie.pdf"), "existing");
    let r = job::duplicate(&[f.root.join("src/r.pdf")], &mut Script::new(vec![]), &AtomicBool::new(false), &f.cfg);
    assert_eq!(r.results, vec![f.root.join("src/r copie (1).pdf")]);
    assert_eq!(read(&f.root.join("src/r copie.pdf")), "existing");
}

#[test]
fn rename_mkdir_trash_with_undo() {
    let f = fixture();
    let a = f.root.join("src/a.txt");
    write(&a, "A");
    write(&f.root.join("src/b.txt"), "B");
    // rename refuses a taken name, accepts a free one, ignores unchanged
    assert!(matches!(rename(&a, "b.txt"), Err(OpError::Exists(_))));
    assert!(rename(&a, "a.txt").unwrap().is_none());
    let (renamed, undo) = rename(&a, "c/d.txt").unwrap().unwrap();
    assert_eq!(renamed, f.root.join("src/c\u{2215}d.txt"));
    assert!(undo.undo(false).is_empty());
    assert!(a.exists());
    // mkdir
    let name = new_folder_name(&f.root.join("src"));
    assert_eq!(name, "Nouveau dossier");
    let (d, undo) = mkdir(&f.root.join("src"), &name).unwrap();
    assert!(d.is_dir());
    assert_eq!(new_folder_name(&f.root.join("src")), "Nouveau dossier (1)");
    assert!(undo.undo(false).is_empty() && !d.exists());
    // trash + undo
    let rep = trash_items(&[a.clone(), f.root.join("src/b.txt")], &f.cfg);
    assert_eq!(rep.trashed.len(), 2);
    assert!(!a.exists());
    assert!(rep.undo().unwrap().undo(false).is_empty());
    assert_eq!(read(&a), "A");
}

#[test]
fn delete_permanently_spares_symlink_targets() {
    let f = fixture();
    write(&f.root.join("outside/keep"), "k");
    fs::create_dir_all(f.root.join("src/d")).unwrap();
    std::os::unix::fs::symlink(f.root.join("outside"), f.root.join("src/d/l")).unwrap();
    assert!(delete_permanently(&[f.root.join("src/d")]).is_empty());
    assert!(!f.root.join("src/d").exists());
    assert_eq!(read(&f.root.join("outside/keep")), "k");
}

/// Cross-filesystem move, if /dev/shm is a different filesystem than the temp dir.
#[test]
fn move_across_filesystems_when_available() {
    let f = fixture();
    let shm = Path::new("/dev/shm");
    if !shm.is_dir() || fsutil::same_device(shm, &f.root) {
        return;
    }
    let other = tempfile::tempdir_in(shm).unwrap();
    write(&f.root.join("src/d/x"), "x");
    write(&f.root.join("src/d/y/z"), "z");
    let r = job::transfer(TransferKind::Move, &[f.root.join("src/d")], other.path(), &mut Script::new(vec![]), &AtomicBool::new(false), &f.cfg);
    assert_eq!(read(&other.path().join("d/y/z")), "z");
    assert!(!f.root.join("src/d").exists());
    assert!(no_part_files(other.path()));
    assert!(r.undo.unwrap().undo(false).is_empty());
    assert_eq!(read(&f.root.join("src/d/y/z")), "z");
    assert!(!other.path().join("d").exists());
}
