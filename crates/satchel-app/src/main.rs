//! Satchel — application shell: state, controllers, bridge to the Slint UI.
//! Milestone 1 (visual prototype): read-only browsing. See docs/ROADMAP.md.

slint::include_modules!();

mod kinds;
mod thumbs;

use satchel_core::places::{self, Device, Place};
use satchel_core::{Entry, SortKey, format, list_dir, sort_entries, volume};
use slint::{ComponentHandle, Image, Model, ModelRc, SharedString, VecModel, Weak};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

fn trace_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("SATCHEL_TRACE").is_some())
}

macro_rules! trace {
    ($($t:tt)*) => { if trace_enabled() { eprintln!("[trace] {}", format!($($t)*)); } };
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// Runs `f` on the app state (UI thread only).
fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|a| a.borrow_mut().as_mut().map(f))
}

thread_local! {
    /// Sidebar model, created once and updated in place.
    static SIDEBAR: Rc<VecModel<SidebarItem>> = Rc::new(VecModel::default());
}

/// Updates a model in place: rows that differ are replaced; length changes add/remove at the end.
/// Never resets the whole model, so Slint keeps the existing item instances (and any mouse grab).
fn update_model<T: Clone + PartialEq + 'static>(model: &VecModel<T>, new: Vec<T>) {
    let old_len = model.row_count();
    for (i, item) in new.iter().enumerate() {
        if i < old_len {
            if model.row_data(i).as_ref() != Some(item) {
                model.set_row_data(i, item.clone());
            }
        } else {
            model.push(item.clone());
        }
    }
    for i in (new.len()..old_len).rev() {
        model.remove(i);
    }
}

/// Runs `f` after the current input event has been fully dispatched. State changes triggered by a
/// pointer event must not rebuild the item tree while Slint is still delivering that event.
fn defer(f: impl FnOnce() + 'static) {
    slint::Timer::single_shot(Duration::ZERO, f);
}

const TYPEAHEAD_TIMEOUT: Duration = Duration::from_millis(1000); // spec 01 §2.5

struct Pane {
    path: PathBuf,
    back: Vec<PathBuf>,
    fwd: Vec<PathBuf>,
    /// Sorted listing (unfiltered).
    all: Vec<Entry>,
    /// Indices into `all` currently shown (after filter).
    visible: Vec<usize>,
    rows: Rc<VecModel<FileRow>>,
    crumbs: Rc<VecModel<Crumb>>,
    selected: HashSet<PathBuf>,
    current: Option<usize>,
    anchor: Option<usize>,
    sort_key: SortKey,
    asc: bool,
    view_mode: i32,
    show_hidden: bool,
    filter: String,
    request: u64,
    message: String,
    free: Option<u64>,
    thumbs: HashMap<PathBuf, Image>,
    select_after_load: Option<PathBuf>,
}

impl Pane {
    fn new(path: PathBuf, view_mode: i32) -> Self {
        Pane {
            path,
            back: Vec::new(),
            fwd: Vec::new(),
            all: Vec::new(),
            visible: Vec::new(),
            rows: Rc::new(VecModel::default()),
            crumbs: Rc::new(VecModel::default()),
            selected: HashSet::new(),
            current: None,
            anchor: None,
            sort_key: SortKey::Name,
            asc: true,
            view_mode,
            show_hidden: false,
            filter: String::new(),
            request: 0,
            message: String::new(),
            free: None,
            thumbs: HashMap::new(),
            select_after_load: None,
        }
    }

    fn entry(&self, vi: usize) -> Option<&Entry> {
        self.visible.get(vi).map(|&i| &self.all[i])
    }

    fn make_row(&self, vi: usize) -> FileRow {
        let e = &self.all[self.visible[vi]];
        let thumb = self.thumbs.get(&e.path).cloned();
        FileRow {
            name: e.name.as_str().into(),
            size: if e.is_dir { format::items(e.size).into() } else { format::size(e.size).into() },
            date: e.modified.map(format::date).unwrap_or_default().into(),
            kind: kinds::kind(e).into(),
            is_dir: e.is_dir,
            selected: self.selected.contains(&e.path),
            current: self.current == Some(vi),
            has_thumb: thumb.is_some(),
            thumb: thumb.unwrap_or_default(),
        }
    }

    /// Recomputes `visible` from `all` + filter and replaces the model rows.
    fn rebuild(&mut self) {
        let needle = self.filter.to_lowercase();
        self.visible = (0..self.all.len())
            .filter(|&i| needle.is_empty() || self.all[i].name.to_lowercase().contains(&needle))
            .collect();
        if let Some(c) = self.current {
            if c >= self.visible.len() {
                self.current = if self.visible.is_empty() { None } else { Some(self.visible.len() - 1) };
            }
        }
        let rows: Vec<FileRow> = (0..self.visible.len()).map(|vi| self.make_row(vi)).collect();
        self.rows.set_vec(rows);
    }

    /// Cheap refresh of selection/current flags without rebuilding the model.
    fn refresh_flags(&self) {
        for vi in 0..self.rows.row_count() {
            let Some(mut row) = self.rows.row_data(vi) else { continue };
            let e = &self.all[self.visible[vi]];
            let sel = self.selected.contains(&e.path);
            let cur = self.current == Some(vi);
            if row.selected != sel || row.current != cur {
                row.selected = sel;
                row.current = cur;
                self.rows.set_row_data(vi, row);
            }
        }
    }

    fn resort(&mut self) {
        sort_entries(&mut self.all, self.sort_key, self.asc, true, false);
        let current_path = self.current.and_then(|c| self.entry(c)).map(|e| e.path.clone());
        self.rebuild();
        if let Some(p) = current_path {
            self.current = self.visible.iter().position(|&i| self.all[i].path == p);
        }
        self.refresh_flags();
    }

    fn selected_entries(&self) -> Vec<&Entry> {
        self.visible.iter().map(|&i| &self.all[i]).filter(|e| self.selected.contains(&e.path)).collect()
    }

    fn status(&self) -> String {
        let free = self.free.map(|f| format!(", {} disponibles", format::size(f))).unwrap_or_default();
        let n = self.visible.len() as u64;
        if !self.filter.is_empty() {
            return format!("{} sur {} correspondent à « {} »", n, format::items(self.all.len() as u64), self.filter);
        }
        let sel = self.selected_entries();
        if sel.is_empty() {
            format!("{}{}", format::items(n), free)
        } else {
            let bytes: u64 = sel.iter().filter(|e| !e.is_dir).map(|e| e.size).sum();
            let word = if sel.len() > 1 { "sélectionnés" } else { "sélectionné" };
            format!("{} sur {} {} ({}){}", sel.len(), n, word, format::size(bytes), free)
        }
    }
}

struct App {
    ui: Weak<MainWindow>,
    panes: [Pane; 2],
    active: usize,
    home: PathBuf,
    favorites: Vec<Place>,
    devices: Vec<Device>,
    typeahead: String,
    typeahead_at: Instant,
    preview_request: u64,
}

impl App {
    fn ui(&self) -> MainWindow {
        self.ui.upgrade().expect("UI alive")
    }

    // ------------------------------------------------------------------ loading

    fn load(&mut self, p: usize) {
        let pane = &mut self.panes[p];
        pane.request += 1;
        let request = pane.request;
        let path = pane.path.clone();
        let show_hidden = pane.show_hidden;
        std::thread::spawn(move || {
            let result = list_dir(&path, show_hidden);
            let free = volume::available_space(&path);
            let _ = slint::invoke_from_event_loop(move || {
                with_app(|app| app.on_listed(p, request, result, free));
            });
        });
    }

    fn on_listed(&mut self, p: usize, request: u64, result: std::io::Result<Vec<Entry>>, free: Option<u64>) {
        let pane = &mut self.panes[p];
        if pane.request != request {
            return; // stale
        }
        pane.free = free;
        match result {
            Ok(entries) => {
                pane.message.clear();
                pane.all = entries;
            }
            Err(err) => {
                pane.message = format!("Impossible d'ouvrir ce dossier : {err}");
                pane.all.clear();
            }
        }
        pane.thumbs.clear();
        pane.current = None;
        pane.anchor = None;
        sort_entries(&mut pane.all, pane.sort_key, pane.asc, true, false);
        pane.rebuild();
        if let Some(target) = pane.select_after_load.take() {
            if let Some(vi) = pane.visible.iter().position(|&i| pane.all[i].path == target) {
                pane.current = Some(vi);
                pane.anchor = Some(vi);
                pane.selected.insert(target);
                pane.refresh_flags();
            }
        } else if !pane.visible.is_empty() {
            pane.current = Some(0);
            pane.refresh_flags();
        }
        self.request_thumbnails(p);
        self.sync_pane(p);
        self.sync_inspector();
    }

    fn request_thumbnails(&self, p: usize) {
        let pane = &self.panes[p];
        let jobs: Vec<PathBuf> = pane.all.iter().filter(|e| !e.is_dir && kinds::is_image(e)).map(|e| e.path.clone()).collect();
        if jobs.is_empty() {
            return;
        }
        let request = pane.request;
        thumbs::spawn_batch(jobs, 256, move |path, buf| {
            with_app(|app| app.on_thumbnail(p, request, path, buf));
        });
    }

    fn on_thumbnail(&mut self, p: usize, request: u64, path: PathBuf, buf: thumbs::Pixels) {
        let pane = &mut self.panes[p];
        if pane.request != request {
            return;
        }
        let img = Image::from_rgba8(buf);
        pane.thumbs.insert(path.clone(), img.clone());
        if let Some(vi) = pane.visible.iter().position(|&i| pane.all[i].path == path) {
            if let Some(mut row) = pane.rows.row_data(vi) {
                row.has_thumb = true;
                row.thumb = img;
                pane.rows.set_row_data(vi, row);
            }
        }
    }

    // --------------------------------------------------------------- navigation

    fn navigate(&mut self, p: usize, path: PathBuf, push_history: bool) {
        let pane = &mut self.panes[p];
        if path == pane.path && push_history {
            return;
        }
        if push_history {
            let old = std::mem::replace(&mut pane.path, path);
            pane.back.push(old);
            pane.fwd.clear();
        } else {
            pane.path = path;
        }
        pane.selected.clear();
        pane.filter.clear();
        if p == self.active {
            self.ui().set_search_text(SharedString::new());
        }
        self.load(p);
        self.sync_pane(p);
        self.sync_chrome();
    }

    fn go_back(&mut self) {
        let p = self.active;
        let pane = &mut self.panes[p];
        if let Some(prev) = pane.back.pop() {
            let cur = std::mem::replace(&mut pane.path, prev.clone());
            pane.fwd.push(cur.clone());
            pane.path = cur; // navigate() swaps it in
            pane.select_after_load = None;
            self.navigate(p, prev, false);
        }
    }

    fn go_forward(&mut self) {
        let p = self.active;
        let pane = &mut self.panes[p];
        if let Some(next) = pane.fwd.pop() {
            let cur = pane.path.clone();
            pane.back.push(cur);
            self.navigate(p, next, false);
        }
    }

    fn go_up(&mut self) {
        let p = self.active;
        let cur = self.panes[p].path.clone();
        if let Some(parent) = cur.parent() {
            self.panes[p].select_after_load = Some(cur.clone());
            self.navigate(p, parent.to_path_buf(), true);
        }
    }

    fn activate(&mut self, p: usize, vi: usize) {
        let Some(e) = self.panes[p].entry(vi).cloned() else { return };
        if e.is_dir {
            self.navigate(p, e.path, true);
        } else if let Err(err) = std::process::Command::new("xdg-open").arg(&e.path).spawn() {
            self.panes[p].message = format!("Impossible d'ouvrir « {} » : {err}", e.name);
            self.sync_pane(p);
        }
    }

    // ---------------------------------------------------------------- selection

    fn press(&mut self, p: usize, vi: usize, ctrl: bool, shift: bool) {
        let pane = &mut self.panes[p];
        let Some(path) = pane.entry(vi).map(|e| e.path.clone()) else { return };
        if ctrl {
            if !pane.selected.remove(&path) {
                pane.selected.insert(path);
            }
            pane.anchor = Some(vi);
        } else if shift {
            // spec 01 §1.4: the range from the anchor is added to the existing selection
            let anchor = pane.anchor.unwrap_or(vi);
            let (a, b) = if anchor <= vi { (anchor, vi) } else { (vi, anchor) };
            for i in a..=b {
                let p = pane.all[pane.visible[i]].path.clone();
                pane.selected.insert(p);
            }
        } else {
            pane.selected.clear();
            pane.selected.insert(path);
            pane.anchor = Some(vi);
        }
        pane.current = Some(vi);
        pane.refresh_flags();
        self.sync_pane(p);
        self.sync_inspector();
    }

    fn move_current(&mut self, delta: isize, shift: bool, absolute: Option<usize>) {
        let p = self.active;
        let pane = &mut self.panes[p];
        let n = pane.visible.len();
        if n == 0 {
            return;
        }
        let target = match absolute {
            Some(t) => t.min(n - 1),
            None => match pane.current {
                None => 0,
                Some(c) => (c as isize + delta).clamp(0, n as isize - 1) as usize,
            },
        };
        if shift {
            let anchor = *pane.anchor.get_or_insert(pane.current.unwrap_or(target));
            pane.selected.clear();
            let (a, b) = if anchor <= target { (anchor, target) } else { (target, anchor) };
            for i in a..=b {
                pane.selected.insert(pane.all[pane.visible[i]].path.clone());
            }
        } else {
            pane.selected.clear();
            pane.selected.insert(pane.all[pane.visible[target]].path.clone());
            pane.anchor = Some(target);
        }
        pane.current = Some(target);
        pane.refresh_flags();
        self.sync_pane(p);
        self.sync_inspector();
    }

    /// Type-ahead: prefix match, case/accent-insensitive; repeating one letter cycles (spec 01 §2.5).
    fn typeahead(&mut self, text: &str) {
        let now = Instant::now();
        if now.duration_since(self.typeahead_at) > TYPEAHEAD_TIMEOUT {
            self.typeahead.clear();
        }
        self.typeahead_at = now;
        self.typeahead.push_str(&text.to_lowercase());
        let pane = &self.panes[self.active];
        let n = pane.visible.len();
        if n == 0 {
            return;
        }
        let buf = &self.typeahead;
        let first = buf.chars().next().unwrap();
        let cycling = buf.chars().all(|c| c == first) && buf.chars().count() > 1;
        let needle: String = if cycling { first.to_string() } else { buf.clone() };
        let start = pane.current.map(|c| if cycling { c + 1 } else { c }).unwrap_or(0);
        let found = (0..n).map(|k| (start + k) % n).find(|&vi| {
            let name = kinds::fold(&pane.all[pane.visible[vi]].name);
            name.starts_with(&kinds::fold(&needle))
        });
        if let Some(vi) = found {
            self.move_current(0, false, Some(vi));
        }
    }

    fn handle_key(&mut self, name: &str, ctrl: bool, shift: bool, alt: bool) -> bool {
        let p = self.active;
        match (name, ctrl, alt) {
            ("Tab", false, false) => {
                self.set_active(1 - p);
                true
            }
            ("Up", false, true) | ("Backspace", false, false) => {
                self.go_up();
                true
            }
            ("Left", false, true) => {
                self.go_back();
                true
            }
            ("Right", false, true) => {
                self.go_forward();
                true
            }
            ("Up", false, false) => {
                self.move_current(-1, shift, None);
                true
            }
            ("Down", false, false) => {
                self.move_current(1, shift, None);
                true
            }
            ("Left", false, false) if self.panes[p].view_mode == 0 => {
                self.move_current(-1, shift, None);
                true
            }
            ("Right", false, false) if self.panes[p].view_mode == 0 => {
                self.move_current(1, shift, None);
                true
            }
            ("Home", false, false) => {
                self.move_current(0, shift, Some(0));
                true
            }
            ("End", false, false) => {
                self.move_current(0, shift, Some(usize::MAX));
                true
            }
            ("Return", false, false) => {
                if let Some(c) = self.panes[p].current {
                    self.activate(p, c);
                }
                true
            }
            ("Escape", false, false) => {
                let pane = &mut self.panes[p];
                pane.selected.clear();
                pane.refresh_flags();
                self.sync_pane(p);
                self.sync_inspector();
                true
            }
            ("F11", false, false) => {
                let ui = self.ui();
                ui.set_inspector_visible(!ui.get_inspector_visible());
                true
            }
            (k, true, false) => match k.to_lowercase().as_str() {
                "h" => {
                    let pane = &mut self.panes[p];
                    pane.show_hidden = !pane.show_hidden;
                    self.load(p);
                    true
                }
                "a" => {
                    let pane = &mut self.panes[p];
                    pane.selected = pane.visible.iter().map(|&i| pane.all[i].path.clone()).collect();
                    pane.refresh_flags();
                    self.sync_pane(p);
                    self.sync_inspector();
                    true
                }
                "1" => {
                    self.set_view_mode(0);
                    true
                }
                "2" | "3" => {
                    self.set_view_mode(1);
                    true
                }
                _ => false,
            },
            (k, false, false) if k.chars().count() == 1 && k.chars().all(|c| !c.is_control() && c != ' ' || c == ' ') => {
                if k.chars().all(|c| c.is_control()) {
                    return false;
                }
                self.typeahead(k);
                true
            }
            _ => false,
        }
    }

    fn set_active(&mut self, p: usize) {
        if self.active == p {
            return;
        }
        self.active = p;
        let filter = self.panes[p].filter.clone();
        let ui = self.ui();
        ui.set_active_pane(p as i32);
        ui.set_search_text(filter.into());
        self.sync_chrome();
        self.sync_inspector();
    }

    fn set_view_mode(&mut self, mode: i32) {
        let p = self.active;
        self.panes[p].view_mode = mode;
        self.sync_pane(p);
    }

    fn sort_by(&mut self, p: usize, key: i32) {
        let key = match key {
            1 => SortKey::Size,
            2 => SortKey::Modified,
            _ => SortKey::Name,
        };
        let pane = &mut self.panes[p];
        if pane.sort_key == key {
            pane.asc = !pane.asc;
        } else {
            pane.sort_key = key;
            // spec 01 §4.4: size and date default to descending
            pane.asc = key == SortKey::Name;
        }
        pane.resort();
        self.sync_pane(p);
    }

    fn search_changed(&mut self, text: &str) {
        let p = self.active;
        let pane = &mut self.panes[p];
        pane.filter = text.trim().to_string();
        pane.rebuild();
        pane.refresh_flags();
        self.sync_pane(p);
    }

    // ------------------------------------------------------------------ syncing

    fn sync_pane(&self, p: usize) {
        let pane = &self.panes[p];
        let (crumbs, root_icon) = crumbs(&pane.path, &self.home);
        let data = PaneData {
            root_icon: root_icon.into(),
            status: pane.status().into(),
            title: title(&pane.path, &self.home).into(),
            sort_key: match pane.sort_key {
                SortKey::Name => 0,
                SortKey::Size => 1,
                SortKey::Modified => 2,
            },
            sort_asc: pane.asc,
            current: pane.current.map(|c| c as i32).unwrap_or(-1),
            message: pane.message.as_str().into(),
            empty: pane.visible.is_empty() && pane.message.is_empty(),
        };
        update_model(&pane.crumbs, crumbs);
        let ui = self.ui();
        if p == 0 {
            ui.set_left_mode(pane.view_mode);
            ui.set_left(data);
        } else {
            ui.set_right_mode(pane.view_mode);
            ui.set_right(data);
        }
        if p == self.active {
            ui.set_can_back(!pane.back.is_empty());
            ui.set_can_forward(!pane.fwd.is_empty());
        }
    }

    fn sync_chrome(&self) {
        let pane = &self.panes[self.active];
        let ui = self.ui();
        ui.set_title_text(title(&pane.path, &self.home).into());
        ui.set_can_back(!pane.back.is_empty());
        ui.set_can_forward(!pane.fwd.is_empty());
        let items = self.sidebar_items();
        SIDEBAR.with(|m| update_model(m, items));
    }

    fn sidebar_items(&self) -> Vec<SidebarItem> {
        let active = &self.panes[self.active].path;
        let mut out = Vec::new();
        let header = |label: &str| SidebarItem { header: true, label: label.into(), ..Default::default() };
        out.push(header("Appareils"));
        for d in &self.devices {
            out.push(SidebarItem {
                header: false,
                label: d.label.as_str().into(),
                path: d.mount_point.to_string_lossy().as_ref().into(),
                detail: volume::available_space(&d.mount_point).map(format::size).unwrap_or_default().into(),
                icon: "drive".into(),
                selected: &d.mount_point == active,
            });
        }
        out.push(header("Favoris"));
        for f in &self.favorites {
            out.push(SidebarItem {
                header: false,
                label: f.label.as_str().into(),
                path: f.path.to_string_lossy().as_ref().into(),
                detail: SharedString::new(),
                icon: kinds::place_icon(&f.icon).into(),
                selected: &f.path == active,
            });
        }
        out
    }

    fn sync_inspector(&mut self) {
        let pane = &self.panes[self.active];
        let sel = pane.selected_entries();
        let ui = self.ui();
        self.preview_request += 1;
        let data = match sel.as_slice() {
            [] => {
                let name = title(&pane.path, &self.home);
                let meta = std::fs::metadata(&pane.path).ok();
                InspectorData {
                    mode: 2,
                    name: name.into(),
                    kind_text: format!("Dossier — {}", format::items(pane.all.len() as u64)).into(),
                    kind: "folder".into(),
                    path: pane.path.to_string_lossy().as_ref().into(),
                    size: format::items(pane.all.len() as u64).into(),
                    modified: meta.as_ref().and_then(|m| m.modified().ok()).map(format::date).unwrap_or_default().into(),
                    created: meta.as_ref().and_then(|m| m.created().ok()).map(format::date).unwrap_or_default().into(),
                    ..Default::default()
                }
            }
            [e] => {
                let meta = std::fs::metadata(&e.path).ok();
                let size = if e.is_dir {
                    format::items(e.size)
                } else {
                    format!("{} ({} octets)", format::size(e.size), format::grouped(e.size))
                };
                let ext = if e.is_dir { String::new() } else { kinds::extension(&e.name).to_string() };
                let kind_text = if e.is_dir {
                    format!("Dossier — {}", format::items(e.size))
                } else {
                    format!("{} — {}", kinds::kind_text(e), format::size(e.size))
                };
                let preview = pane.thumbs.get(&e.path).cloned();
                if !e.is_dir && kinds::is_image(e) {
                    let request = self.preview_request;
                    let path = e.path.clone();
                    thumbs::spawn_batch(vec![path], 720, move |_, buf| {
                        with_app(|app| {
                            if app.preview_request == request {
                                let ui = app.ui();
                                let mut d = ui.get_inspector();
                                d.preview = Image::from_rgba8(buf);
                                d.has_preview = true;
                                ui.set_inspector(d);
                            }
                        });
                    });
                }
                InspectorData {
                    mode: 0,
                    name: e.name.as_str().into(),
                    kind_text: kind_text.into(),
                    kind: kinds::kind(e).into(),
                    path: e.path.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default().into(),
                    size: size.into(),
                    modified: e.modified.map(format::date).unwrap_or_default().into(),
                    created: meta.as_ref().and_then(|m| m.created().ok()).map(format::date).unwrap_or_default().into(),
                    extension: ext.into(),
                    hidden: e.hidden,
                    has_preview: preview.is_some(),
                    preview: preview.unwrap_or_default(),
                }
            }
            many => {
                let bytes: u64 = many.iter().filter(|e| !e.is_dir).map(|e| e.size).sum();
                InspectorData {
                    mode: 1,
                    name: format::items(many.len() as u64).into(),
                    kind_text: format::size(bytes).into(),
                    kind: "doc".into(),
                    ..Default::default()
                }
            }
        };
        ui.set_inspector(data);
    }
}

fn title(path: &Path, home: &Path) -> String {
    if path == Path::new("/") {
        return "Système".into();
    }
    if path == home {
        return home.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "Dossier personnel".into());
    }
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// Breadcrumb from the home folder (if inside it) or from the root.
fn crumbs(path: &Path, home: &Path) -> (Vec<Crumb>, &'static str) {
    let mut out = Vec::new();
    let (base, rest, icon) = match path.strip_prefix(home) {
        Ok(rest) => (home.to_path_buf(), rest.to_path_buf(), "home"),
        Err(_) => (PathBuf::from("/"), path.strip_prefix("/").unwrap_or(path).to_path_buf(), "drive"),
    };
    out.push(Crumb { label: title(&base, home).into(), path: base.to_string_lossy().as_ref().into() });
    let mut acc = base;
    for comp in rest.components() {
        acc.push(comp);
        out.push(Crumb {
            label: comp.as_os_str().to_string_lossy().as_ref().into(),
            path: acc.to_string_lossy().as_ref().into(),
        });
    }
    (out, icon)
}

fn default_favorites(home: &Path) -> Vec<Place> {
    let mut v = vec![Place { label: "Dossier personnel".into(), path: home.to_path_buf(), icon: "user-home".into() }];
    for (dir, label, icon) in [
        ("Bureau", "Bureau", "user-desktop"),
        ("Documents", "Documents", "folder-documents"),
        ("Téléchargements", "Téléchargements", "folder-download"),
        ("Images", "Images", "folder-pictures"),
        ("Musique", "Musique", "folder-music"),
        ("Vidéos", "Vidéos", "folder-videos"),
    ] {
        let p = home.join(dir);
        if p.is_dir() {
            v.push(Place { label: label.into(), path: p, icon: icon.into() });
        }
    }
    v
}

/// After `drag_window`/`drag_resize_window` the compositor owns the pointer and the button release
/// never reaches Slint, which would keep the drag TouchArea grabbed forever (every later click would
/// restart a move). Synthesize the release once the current event is done.
fn release_pointer_grab(ui: &MainWindow) {
    let w = ui.as_weak();
    defer(move || {
        if let Some(ui) = w.upgrade() {
            trace!("synthetic pointer release after compositor grab");
            ui.window().dispatch_event(slint::platform::WindowEvent::PointerReleased {
                position: slint::LogicalPosition::new(0.0, 0.0),
                button: slint::platform::PointerEventButton::Left,
            });
            ui.window().dispatch_event(slint::platform::WindowEvent::PointerExited);
        }
    });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use slint::winit_030::WinitWindowAccessor;
    use slint::winit_030::winit::window::ResizeDirection;

    slint::BackendSelector::new().backend_name("winit".into()).select()?;

    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| "/".into()));
    let ui = MainWindow::new()?;

    let mut favorites = places::read_xbel(&home.join(".local/share/user-places.xbel"));
    if favorites.is_empty() {
        favorites = default_favorites(&home);
    }
    let pictures = ["Images", "Pictures"].iter().map(|d| home.join(d)).find(|p| p.is_dir()).unwrap_or_else(|| home.clone());
    let start_left = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| home.clone());

    APP.with(|a| {
        *a.borrow_mut() = Some(App {
            ui: ui.as_weak(),
            panes: [Pane::new(start_left, 1), Pane::new(pictures, 0)],
            active: 0,
            home: home.clone(),
            favorites,
            devices: places::mounted_devices(),
            typeahead: String::new(),
            typeahead_at: Instant::now(),
            preview_request: 0,
        })
    });

    if trace_enabled() {
        use slint::winit_030::{winit::event::WindowEvent, EventResult, WinitWindowAccessor};
        ui.window().on_winit_window_event(|_, ev| {
            match ev {
                WindowEvent::MouseInput { state, button, .. } => eprintln!("[trace] winit MouseInput {state:?} {button:?}"),
                WindowEvent::CursorEntered { .. } => eprintln!("[trace] winit CursorEntered"),
                WindowEvent::Focused(f) => eprintln!("[trace] winit Focused({f})"),
                WindowEvent::KeyboardInput { event, .. } => eprintln!("[trace] winit Key {:?} {:?}", event.logical_key, event.state),
                _ => {}
            }
            EventResult::Propagate
        });
    }

    // Models are bound once; afterwards they are only updated in place (see update_model).
    with_app(|a| {
        let ui = a.ui();
        ui.set_left_rows(ModelRc::from(a.panes[0].rows.clone()));
        ui.set_right_rows(ModelRc::from(a.panes[1].rows.clone()));
        ui.set_left_crumbs(ModelRc::from(a.panes[0].crumbs.clone()));
        ui.set_right_crumbs(ModelRc::from(a.panes[1].crumbs.clone()));
        SIDEBAR.with(|m| ui.set_sidebar(ModelRc::from(m.clone())));
    });

    // Pointer-driven callbacks are deferred until the event has been dispatched (see `defer`).
    ui.on_row_pressed(|p, i, ctrl, shift| {
        trace!("row_pressed pane={p} row={i} ctrl={ctrl} shift={shift}");
        defer(move || {
            with_app(|a| a.press(p as usize, i as usize, ctrl, shift));
        });
    });
    ui.on_row_activated(|p, i| {
        trace!("row_activated pane={p} row={i}");
        defer(move || {
            with_app(|a| a.activate(p as usize, i as usize));
        });
    });
    ui.on_navigate(|p, path| {
        trace!("navigate pane={p} {path}");
        defer(move || {
            with_app(|a| a.navigate(p as usize, PathBuf::from(path.as_str()), true));
        });
    });
    ui.on_sort_by(|p, k| {
        defer(move || {
            with_app(|a| a.sort_by(p as usize, k));
        });
    });
    ui.on_set_active(|p| {
        trace!("set_active {p}");
        defer(move || {
            with_app(|a| a.set_active(p as usize));
        });
    });
    ui.on_sidebar_clicked(|path| {
        trace!("sidebar_clicked {path}");
        defer(move || {
            with_app(|a| {
                let p = a.active;
                a.navigate(p, PathBuf::from(path.as_str()), true);
            });
        });
    });
    ui.on_go_back(|| {
        defer(|| {
            with_app(|a| a.go_back());
        });
    });
    ui.on_go_forward(|| {
        defer(|| {
            with_app(|a| a.go_forward());
        });
    });
    ui.on_set_view_mode(|m| {
        defer(move || {
            with_app(|a| a.set_view_mode(m));
        });
    });
    ui.on_search_changed(|t| {
        with_app(|a| a.search_changed(t.as_str()));
    });
    ui.on_key(|name, ctrl, shift, alt| {
        trace!("key {name:?} ctrl={ctrl} shift={shift} alt={alt}");
        with_app(|a| a.handle_key(name.as_str(), ctrl, shift, alt)).unwrap_or(false)
    });
    ui.on_todo(|what| {
        with_app(|a| {
            let p = a.active;
            a.panes[p].message = format!("Pas encore disponible : {what} (jalon 2)");
            a.sync_pane(p);
        });
    });

    let w = ui.as_weak();
    ui.on_close_window(move || {
        if let Some(ui) = w.upgrade() {
            let _ = ui.hide();
        }
        let _ = slint::quit_event_loop();
    });
    let w = ui.as_weak();
    ui.on_minimize_window(move || {
        if let Some(ui) = w.upgrade() {
            ui.window().set_minimized(true);
        }
    });
    let w = ui.as_weak();
    ui.on_maximize_window(move || {
        if let Some(ui) = w.upgrade() {
            let m = !ui.window().is_maximized();
            ui.window().set_maximized(m);
        }
    });
    let w = ui.as_weak();
    ui.on_start_drag(move || {
        if let Some(ui) = w.upgrade() {
            ui.window().with_winit_window(|win| {
                let _ = win.drag_window();
            });
            release_pointer_grab(&ui);
        }
    });
    let w = ui.as_weak();
    ui.on_start_resize(move |dir| {
        let dir = match dir {
            0 => ResizeDirection::East,
            1 => ResizeDirection::North,
            2 => ResizeDirection::NorthEast,
            3 => ResizeDirection::NorthWest,
            4 => ResizeDirection::South,
            5 => ResizeDirection::SouthEast,
            6 => ResizeDirection::SouthWest,
            _ => ResizeDirection::West,
        };
        if let Some(ui) = w.upgrade() {
            ui.window().with_winit_window(|win| {
                let _ = win.drag_resize_window(dir);
            });
            release_pointer_grab(&ui);
        }
    });

    with_app(|a| {
        a.sync_chrome();
        a.sync_pane(0);
        a.sync_pane(1);
        a.load(0);
        a.load(1);
    });

    ui.run()?;
    Ok(())
}
