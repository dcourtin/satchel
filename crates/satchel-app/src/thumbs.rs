//! Background thumbnail decoding. Results are delivered on the UI thread.
use slint::{Rgba8Pixel, SharedPixelBuffer};
use std::path::PathBuf;

pub type Pixels = SharedPixelBuffer<Rgba8Pixel>;

/// Skip decoding originals bigger than this (the freedesktop cache is still used).
const MAX_ORIGINAL_BYTES: u64 = 60 * 1024 * 1024;

/// Decodes `paths` on a worker thread (cache first, original second) and calls `on_ready`
/// on the UI thread for each success.
pub fn spawn_batch<F>(paths: Vec<PathBuf>, max_px: u32, on_ready: F)
where
    F: Fn(PathBuf, Pixels) + Send + Clone + 'static,
{
    std::thread::spawn(move || {
        for path in paths {
            let Some(buf) = decode(&path, max_px) else { continue };
            let cb = on_ready.clone();
            let p = path.clone();
            if slint::invoke_from_event_loop(move || cb(p, buf)).is_err() {
                return; // event loop gone
            }
        }
    });
}

fn decode(path: &std::path::Path, max_px: u32) -> Option<Pixels> {
    let source = match satchel_core::thumbs::cached(path) {
        Some(c) if max_px <= 256 => c,
        _ => {
            let len = std::fs::metadata(path).ok()?.len();
            if len > MAX_ORIGINAL_BYTES {
                satchel_core::thumbs::cached(path)?
            } else {
                path.to_path_buf()
            }
        }
    };
    let img = image::open(&source).ok()?;
    let img = if img.width() > max_px || img.height() > max_px { img.thumbnail(max_px, max_px) } else { img };
    let rgba = img.to_rgba8();
    Some(SharedPixelBuffer::clone_from_slice(rgba.as_raw(), rgba.width(), rgba.height()))
}
