//! File-operation controller (Jalon 2): wires `satchel_core::ops` to the UI.
//! Transfers run on a worker thread; conflicts/errors are asked through the modal dialog and the
//! worker blocks on a channel until the user answers. Every disk write goes through `ops`.

use crate::{App, DialogData, JobData, with_app};
use satchel_core::format;
use satchel_core::ops::{self, Conflict, Decision, ErrorAction, OpError, Resolution, TransferHandler, TransferKind, TransferReport, TrashConfig, UndoRecord, UndoStack, job, names};
use slint::ComponentHandle;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

const TOAST_DURATION: Duration = Duration::from_millis(3500);

enum Pending {
    Conflict(mpsc::Sender<Decision>),
    JobError(mpsc::Sender<ErrorAction>),
    LossyOverwrite(mpsc::Sender<bool>),
    NewFolder { pane: usize, dir: PathBuf },
    Rename { pane: usize, path: PathBuf },
    ConfirmDelete { paths: Vec<PathBuf> },
    ConfirmUndo,
    Info,
}

#[derive(Default)]
struct OpsState {
    undo: UndoStack,
    job: Option<Arc<AtomicBool>>,
    pending: Option<Pending>,
    toast_timer: Option<slint::Timer>,
}

thread_local! {
    static OPS: RefCell<OpsState> = RefCell::new(OpsState::default());
}

// ---------------------------------------------------------------- entry points (UI thread)

/// Keyboard shortcuts for operations. Returns None if the key is not an operation.
pub fn key(app: &mut App, name: &str, ctrl: bool, shift: bool, alt: bool) -> Option<bool> {
    if alt {
        return None;
    }
    let action = match (name, ctrl, shift) {
        ("F5", false, _) => "copy-other",
        ("F6", false, _) => "move-other",
        ("F10", false, false) => "new-folder",
        ("F2", false, false) => "rename",
        ("Delete", false, false) => "trash",
        ("Delete", false, true) => "delete",
        (k, true, false) if k.eq_ignore_ascii_case("d") => "duplicate",
        (k, true, false) if k.eq_ignore_ascii_case("z") => "undo",
        _ => return None,
    };
    action_impl(app, action);
    Some(true)
}

pub fn action(app: &mut App, name: &str) {
    action_impl(app, name);
}

fn action_impl(app: &mut App, name: &str) {
    if dialog_open(app) {
        return;
    }
    if OPS.with(|o| o.borrow().job.is_some()) {
        return toast(app, "Une opération est en cours : patientez ou annulez-la");
    }
    let p = app.active;
    match name {
        "copy-other" | "move-other" => {
            let kind = if name == "copy-other" { TransferKind::Copy } else { TransferKind::Move };
            let sources = targets(app, p);
            let dest = app.panes[1 - p].path.clone();
            if sources.is_empty() {
                return toast(app, "Rien n'est sélectionné");
            }
            if sources.iter().all(|s| s.parent() == Some(dest.as_path())) {
                return toast(app, "Les deux panneaux affichent le même dossier");
            }
            start_transfer(app, kind, sources, dest, 1 - p);
        }
        "duplicate" => {
            let sources = targets(app, p);
            if sources.is_empty() {
                return toast(app, "Rien n'est sélectionné");
            }
            let dest = app.panes[p].path.clone();
            start_job(app, format!("Duplication de {}", count_label(sources.len())), p, TransferKind::Copy, move |h, cancel, cfg| job::duplicate(&sources, h, cancel, cfg), dest);
        }
        "trash" => trash_selection(app, p),
        "delete" => confirm_delete(app, p),
        "new-folder" => {
            let dir = app.panes[p].path.clone();
            let name = ops::new_folder_name(&dir);
            let end = name.len() as i32;
            set_pending(Pending::NewFolder { pane: p, dir });
            show_dialog(app, DialogData {
                kind: 1,
                title: "Nouveau dossier".into(),
                input: name.into(),
                sel_start: 0,
                sel_end: end,
                primary: "Créer".into(),
                cancel: "Annuler".into(),
                ..Default::default()
            });
        }
        "rename" => {
            let sel = targets(app, p);
            match sel.as_slice() {
                [] => toast(app, "Rien n'est sélectionné"),
                [path] => {
                    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    let is_dir = std::fs::symlink_metadata(path).map(|m| m.is_dir()).unwrap_or(false);
                    let range = names::rename_selection(&name, is_dir);
                    let (start, end) = char_range_to_bytes(&name, range);
                    set_pending(Pending::Rename { pane: p, path: path.clone() });
                    show_dialog(app, DialogData {
                        kind: 1,
                        title: "Renommer".into(),
                        input: name.into(),
                        sel_start: start,
                        sel_end: end,
                        primary: "Renommer".into(),
                        cancel: "Annuler".into(),
                        ..Default::default()
                    });
                }
                _ => toast(app, "Renommer plusieurs éléments à la fois : bientôt"),
            }
        }
        "undo" => undo(app),
        _ => toast(app, &format!("Pas encore disponible : {name}")),
    }
}

/// Answer from the modal dialog (deferred by the caller: never runs inside the click event).
pub fn dialog_answer(app: &mut App, action: &str, text: &str, apply_all: bool) {
    let Some(pending) = OPS.with(|o| o.borrow_mut().pending.take()) else {
        return hide_dialog(app);
    };
    match pending {
        Pending::Conflict(tx) => {
            let resolution = match action {
                "overwrite" => Resolution::Overwrite,
                "overwrite-older" => Resolution::OverwriteIfOlder,
                "skip" => Resolution::Skip,
                "rename" => Resolution::Rename(text.to_string()),
                "autorename" => Resolution::AutoRename,
                "merge" => Resolution::Merge,
                _ => Resolution::Cancel,
            };
            hide_dialog(app);
            let _ = tx.send(Decision { resolution, apply_to_all: apply_all });
        }
        Pending::JobError(tx) => {
            hide_dialog(app);
            let _ = tx.send(match action {
                "primary" => ErrorAction::Retry,
                "secondary" => ErrorAction::Skip,
                _ => ErrorAction::Cancel,
            });
        }
        Pending::LossyOverwrite(tx) => {
            hide_dialog(app);
            let _ = tx.send(action == "primary");
        }
        Pending::NewFolder { pane, dir } => {
            if action != "primary" {
                return hide_dialog(app);
            }
            match ops::mkdir(&dir, text) {
                Ok((path, rec)) => {
                    hide_dialog(app);
                    push_undo(rec);
                    reload_all(app, Some((pane, vec![path])));
                }
                Err(e) => retry_input(app, Pending::NewFolder { pane, dir }, text, e),
            }
        }
        Pending::Rename { pane, path } => {
            if action != "primary" {
                return hide_dialog(app);
            }
            match ops::rename(&path, text) {
                Ok(None) => hide_dialog(app),
                Ok(Some((new_path, rec))) => {
                    hide_dialog(app);
                    push_undo(rec);
                    let hidden_now = new_path.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.'));
                    if hidden_now && !app.panes[pane].show_hidden {
                        toast(app, "Renommé : l'élément est maintenant masqué (Ctrl+H pour l'afficher)");
                    }
                    reload_all(app, Some((pane, vec![new_path])));
                }
                Err(e) => retry_input(app, Pending::Rename { pane, path }, text, e),
            }
        }
        Pending::ConfirmDelete { paths } => {
            hide_dialog(app);
            if action == "primary" {
                delete_permanently(app, paths);
            }
        }
        Pending::ConfirmUndo => {
            hide_dialog(app);
            if action == "primary" {
                perform_undo(app);
            }
        }
        Pending::Info => hide_dialog(app),
    }
}

pub fn cancel_job() {
    OPS.with(|o| {
        if let Some(c) = &o.borrow().job {
            c.store(true, Ordering::Relaxed);
        }
    });
}

// ---------------------------------------------------------------- transfers

fn start_transfer(app: &mut App, kind: TransferKind, sources: Vec<PathBuf>, dest: PathBuf, dest_pane: usize) {
    let dest_name = dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "/".into());
    let verb = if kind == TransferKind::Copy { "Copie" } else { "Déplacement" };
    let label = format!("{verb} de {} vers « {dest_name} »", count_label(sources.len()));
    let d = dest.clone();
    start_job(app, label, dest_pane, kind, move |h, cancel, cfg| job::transfer(kind, &sources, &d, h, cancel, cfg), dest);
}

/// Runs `work` on a worker thread with a UI-backed handler, then finishes on the UI thread.
fn start_job<F>(app: &mut App, label: String, result_pane: usize, kind: TransferKind, work: F, _dest: PathBuf)
where
    F: FnOnce(&mut dyn TransferHandler, &AtomicBool, &TrashConfig) -> TransferReport + Send + 'static,
{
    if OPS.with(|o| o.borrow().job.is_some()) {
        return toast(app, "Une opération est déjà en cours");
    }
    let cancel = Arc::new(AtomicBool::new(false));
    OPS.with(|o| o.borrow_mut().job = Some(cancel.clone()));
    app.ui().set_job(JobData { visible: true, label: label.clone().into(), detail: "Préparation…".into(), progress: -1.0 });
    std::thread::spawn(move || {
        let cfg = TrashConfig::from_env();
        let mut handler = UiHandler { last_progress: Instant::now() - Duration::from_secs(1), label };
        let report = work(&mut handler, &cancel, &cfg);
        let _ = slint::invoke_from_event_loop(move || {
            with_app(|a| finish_job(a, kind, report, result_pane));
        });
    });
}

fn finish_job(app: &mut App, kind: TransferKind, report: TransferReport, result_pane: usize) {
    OPS.with(|o| o.borrow_mut().job = None);
    app.ui().set_job(JobData::default());
    let n = report.results.len();
    if let Some(rec) = report.undo {
        push_undo(rec);
    }
    let verb = match kind {
        TransferKind::Copy => if n > 1 { "copiés" } else { "copié" },
        TransferKind::Move => if n > 1 { "déplacés" } else { "déplacé" },
    };
    let mut msg = if report.cancelled { "Opération annulée".to_string() } else { format!("{} {verb}", count_label(n)) };
    if report.skipped > 0 {
        msg.push_str(&format!(" · {} ignoré{}", report.skipped, if report.skipped > 1 { "s" } else { "" }));
    }
    if n > 0 && !report.cancelled {
        msg.push_str(" — Ctrl+Z pour annuler");
    }
    toast(app, &msg);
    // Spec 02 §17 #28: select what arrived, in the destination pane.
    reload_all(app, Some((result_pane, report.results)));
}

/// Worker-side handler: forwards questions to the UI thread and blocks for the answer.
struct UiHandler {
    last_progress: Instant,
    label: String,
}

impl UiHandler {
    fn ask<T: Send + 'static>(show: impl FnOnce(&mut App, mpsc::Sender<T>) + Send + 'static, fallback: T) -> T {
        let (tx, rx) = mpsc::channel();
        let _ = slint::invoke_from_event_loop(move || {
            with_app(|a| show(a, tx));
        });
        rx.recv().unwrap_or(fallback)
    }
}

impl TransferHandler for UiHandler {
    fn conflict(&mut self, c: &Conflict) -> Decision {
        let c = c.clone();
        Self::ask(move |a, tx| show_conflict(a, &c, tx), Decision { resolution: Resolution::Cancel, apply_to_all: false })
    }

    fn error(&mut self, path: &Path, err: &std::io::Error) -> ErrorAction {
        let name = display_name(path);
        let why = ops::describe_io(err);
        Self::ask(
            move |a, tx| {
                set_pending(Pending::JobError(tx));
                show_dialog(a, DialogData {
                    title: format!("Impossible de traiter « {name} »").into(),
                    message: why.into(),
                    primary: "Réessayer".into(),
                    secondary: "Ignorer".into(),
                    cancel: "Tout annuler".into(),
                    ..Default::default()
                });
            },
            ErrorAction::Cancel,
        )
    }

    fn confirm_overwrite_without_trash(&mut self, target: &Path) -> bool {
        let name = display_name(target);
        Self::ask(
            move |a, tx| {
                set_pending(Pending::LossyOverwrite(tx));
                show_dialog(a, DialogData {
                    title: format!("Remplacer « {name} » définitivement ?").into(),
                    message: "Ce disque n'a pas de corbeille : l'élément existant ne pourra pas être récupéré.".into(),
                    danger: true,
                    primary: "Remplacer définitivement".into(),
                    cancel: "Ignorer".into(),
                    ..Default::default()
                });
            },
            false,
        )
    }

    fn progress(&mut self, done: u64, total: u64, current: &Path) {
        if self.last_progress.elapsed() < Duration::from_millis(100) {
            return;
        }
        self.last_progress = Instant::now();
        let label = self.label.clone();
        let detail = format!("{} sur {} — {}", format::size(done), format::size(total), display_name(current));
        let progress = if total > 0 { done as f32 / total as f32 } else { -1.0 };
        let _ = slint::invoke_from_event_loop(move || {
            with_app(|a| {
                if OPS.with(|o| o.borrow().job.is_some()) {
                    a.ui().set_job(JobData { visible: true, label: label.into(), detail: detail.into(), progress });
                }
            });
        });
    }
}

fn describe_item(is_dir: bool, size: u64, modified: Option<std::time::SystemTime>) -> String {
    let date = modified.map(format::date).unwrap_or_default();
    if is_dir { format!("Dossier · modifié le {date}") } else { format!("{} · modifié le {date}", format::size(size)) }
}

fn show_conflict(app: &mut App, c: &Conflict, tx: mpsc::Sender<Decision>) {
    let name = display_name(&c.target);
    let dir = c.target.parent().unwrap_or(Path::new("/"));
    let suggestion = names::suggest_name(dir, &name);
    let (start, end) = char_range_to_bytes(&suggestion, names::rename_selection(&suggestion, c.source_is_dir));
    let title = if c.same_item {
        format!("« {name} » est déjà dans ce dossier")
    } else if c.source_is_dir && c.target_is_dir {
        format!("Le dossier « {name} » existe déjà")
    } else if c.source_is_dir != c.target_is_dir {
        format!("« {name} » existe déjà (dossier et fichier ne peuvent pas se remplacer)")
    } else {
        format!("« {name} » existe déjà")
    };
    let detail = if c.can_overwrite() {
        "Remplacer envoie l'élément existant à la corbeille : Ctrl+Z le restaure."
    } else if c.can_merge() {
        "Fusionner ajoute le contenu au dossier existant ; chaque conflit de fichier vous sera demandé."
    } else {
        ""
    };
    set_pending(Pending::Conflict(tx));
    show_dialog(app, DialogData {
        kind: 2,
        title: title.into(),
        detail: detail.into(),
        input: suggestion.into(),
        sel_start: start,
        sel_end: end,
        target_info: describe_item(c.target_is_dir, c.target_size, c.target_modified).into(),
        source_info: describe_item(c.source_is_dir, c.source_size, c.source_modified).into(),
        source_newer: c.source_is_newer(),
        target_newer: matches!((c.source_modified, c.target_modified), (Some(s), Some(t)) if t > s),
        can_overwrite: c.can_overwrite(),
        can_merge: c.can_merge(),
        multi: true,
        same_item: c.same_item,
        ..Default::default()
    });
}

// ---------------------------------------------------------------- trash / delete

fn trash_selection(app: &mut App, p: usize) {
    let paths = targets(app, p);
    if paths.is_empty() {
        return toast(app, "Rien n'est sélectionné");
    }
    let next = item_after_selection(app, p);
    let report = ops::trash_items(&paths, &TrashConfig::from_env());
    if let Some(rec) = report.undo() {
        push_undo(rec);
    }
    let n = report.trashed.len();
    if n > 0 {
        toast(app, &format!("{} mis à la corbeille — Ctrl+Z pour annuler", count_label(n)));
    }
    reload_all(app, next.map(|n| (p, vec![n])));
    if !report.unavailable.is_empty() {
        // No trash on that filesystem: never delete silently (spec 02 §17 #3).
        let paths = report.unavailable;
        let names = list_names(&paths);
        set_pending(Pending::ConfirmDelete { paths: paths.clone() });
        show_dialog(app, DialogData {
            title: format!("Pas de corbeille pour {}", count_label(paths.len())).into(),
            message: names.into(),
            detail: "Ce disque n'a pas de corbeille. Les supprimer définitivement ? Cette action est irréversible.".into(),
            danger: true,
            primary: "Supprimer définitivement".into(),
            cancel: "Annuler".into(),
            ..Default::default()
        });
    } else if !report.errors.is_empty() {
        show_errors(app, "Certains éléments n'ont pas pu être mis à la corbeille", &report.errors);
    }
}

fn confirm_delete(app: &mut App, p: usize) {
    let paths = targets(app, p);
    if paths.is_empty() {
        return toast(app, "Rien n'est sélectionné");
    }
    set_pending(Pending::ConfirmDelete { paths: paths.clone() });
    show_dialog(app, DialogData {
        title: format!("Supprimer définitivement {} ?", count_label(paths.len())).into(),
        message: list_names(&paths).into(),
        detail: "Cette action est irréversible : les éléments ne passeront pas par la corbeille.".into(),
        danger: true,
        primary: "Supprimer".into(),
        cancel: "Annuler".into(),
        ..Default::default()
    });
}

fn delete_permanently(app: &mut App, paths: Vec<PathBuf>) {
    if OPS.with(|o| o.borrow().job.is_some()) {
        return toast(app, "Une opération est déjà en cours");
    }
    let p = app.active;
    let next = item_after_selection(app, p);
    let cancel = Arc::new(AtomicBool::new(false));
    OPS.with(|o| o.borrow_mut().job = Some(cancel));
    app.ui().set_job(JobData { visible: true, label: format!("Suppression de {}", count_label(paths.len())).into(), detail: String::new().into(), progress: -1.0 });
    std::thread::spawn(move || {
        let errors = ops::delete_permanently(&paths);
        let n = paths.len() - errors.len();
        let _ = slint::invoke_from_event_loop(move || {
            with_app(|a| {
                OPS.with(|o| o.borrow_mut().job = None);
                a.ui().set_job(JobData::default());
                toast(a, &format!("{} supprimé{} définitivement", count_label(n), if n > 1 { "s" } else { "" }));
                reload_all(a, next.map(|x| (p, vec![x])));
                if !errors.is_empty() {
                    show_errors(a, "Certains éléments n'ont pas pu être supprimés", &errors);
                }
            });
        });
    });
}

// ---------------------------------------------------------------- undo

fn push_undo(rec: UndoRecord) {
    OPS.with(|o| o.borrow_mut().undo.push(rec));
}

fn undo(app: &mut App) {
    let info = OPS.with(|o| o.borrow().undo.peek().map(|r| (r.deletes_files(), r.modified_since().len(), r.label())));
    let Some((deletes, modified, label)) = info else {
        return toast(app, "Rien à annuler");
    };
    if deletes {
        // Undoing a copy deletes files: confirm, and warn about edited copies (spec 02 §13).
        set_pending(Pending::ConfirmUndo);
        let detail = if modified > 0 {
            format!("{} modifié{} depuis la copie {} conservé{}.", count_label(modified), if modified > 1 { "s" } else { "" }, if modified > 1 { "seront" } else { "sera" }, if modified > 1 { "s" } else { "" })
        } else {
            String::new()
        };
        show_dialog(app, DialogData {
            title: format!("Annuler : {label} ?").into(),
            message: "Les copies créées seront supprimées.".into(),
            detail: detail.into(),
            danger: true,
            primary: "Supprimer les copies".into(),
            cancel: "Garder".into(),
            ..Default::default()
        });
        return;
    }
    perform_undo(app);
}

fn perform_undo(app: &mut App) {
    let Some(rec) = OPS.with(|o| o.borrow_mut().undo.pop()) else { return };
    let errors = rec.undo(false);
    if errors.is_empty() {
        toast(app, &format!("Annulé : {}", rec.label()));
    } else {
        show_errors(app, &format!("Annulation partielle : {}", rec.label()), &errors);
    }
    reload_all(app, None);
}

// ---------------------------------------------------------------- helpers

fn dialog_open(app: &App) -> bool {
    app.ui().get_dialog().visible
}

fn set_pending(p: Pending) {
    OPS.with(|o| o.borrow_mut().pending = Some(p));
}

fn show_dialog(app: &App, mut d: DialogData) {
    d.visible = true;
    app.ui().set_dialog(d);
}

fn hide_dialog(app: &App) {
    app.ui().set_dialog(DialogData::default());
}

/// Keeps the name dialog open with an error under the field.
fn retry_input(app: &App, pending: Pending, text: &str, err: OpError) {
    let mut d = app.ui().get_dialog();
    d.input = text.into();
    d.sel_start = 0;
    d.sel_end = text.len() as i32;
    d.error = err.to_string().into();
    set_pending(pending);
    app.ui().set_dialog(d);
}

fn show_errors(app: &App, title: &str, errors: &[(PathBuf, std::io::Error)]) {
    let lines: Vec<String> = errors.iter().take(6).map(|(p, e)| format!("• {} : {}", display_name(p), ops::describe_io(e))).collect();
    let more = if errors.len() > 6 { format!("\n… et {} autres", errors.len() - 6) } else { String::new() };
    set_pending(Pending::Info);
    show_dialog(app, DialogData { title: title.into(), message: format!("{}{more}", lines.join("\n")).into(), primary: "OK".into(), ..Default::default() });
}

pub fn toast(app: &mut App, text: &str) {
    let ui = app.ui();
    ui.set_toast(text.into());
    let w = ui.as_weak();
    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::SingleShot, TOAST_DURATION, move || {
        if let Some(ui) = w.upgrade() {
            ui.set_toast(Default::default());
        }
    });
    OPS.with(|o| o.borrow_mut().toast_timer = Some(timer));
}

/// Selected items of pane `p`, or the current item when nothing is selected.
fn targets(app: &App, p: usize) -> Vec<PathBuf> {
    let pane = &app.panes[p];
    let sel: Vec<PathBuf> = pane.selected_entries().iter().map(|e| e.path.clone()).collect();
    if !sel.is_empty() {
        return sel;
    }
    pane.current.and_then(|c| pane.entry(c)).map(|e| vec![e.path.clone()]).unwrap_or_default()
}

/// Item to select after removing the selection: the one following it, else the one before.
fn item_after_selection(app: &App, p: usize) -> Option<PathBuf> {
    let pane = &app.panes[p];
    let removed = targets(app, p);
    let n = pane.visible.len();
    let last = (0..n).rev().find(|&vi| pane.entry(vi).is_some_and(|e| removed.contains(&e.path)))?;
    (last + 1..n).chain((0..last).rev()).filter_map(|vi| pane.entry(vi)).find(|e| !removed.contains(&e.path)).map(|e| e.path.clone())
}

/// Reloads both panes (no directory watching yet), selecting `select` in a pane if given.
fn reload_all(app: &mut App, select: Option<(usize, Vec<PathBuf>)>) {
    for p in 0..2 {
        let keep: Vec<PathBuf> = match &select {
            Some((sp, paths)) if *sp == p => paths.clone(),
            _ => app.panes[p].selected.iter().cloned().collect(),
        };
        app.panes[p].reselect = keep;
        app.load(p);
    }
}

fn display_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string())
}

fn list_names(paths: &[PathBuf]) -> String {
    let mut s: Vec<String> = paths.iter().take(5).map(|p| format!("• {}", display_name(p))).collect();
    if paths.len() > 5 {
        s.push(format!("… et {} autres", paths.len() - 5));
    }
    s.join("\n")
}

fn count_label(n: usize) -> String {
    format::items(n as u64)
}

fn char_range_to_bytes(s: &str, r: std::ops::Range<usize>) -> (i32, i32) {
    let byte = |ci: usize| s.char_indices().nth(ci).map(|(b, _)| b).unwrap_or(s.len()) as i32;
    (byte(r.start), byte(r.end))
}
