//! Satchel core: filesystem model with no UI dependency.
//! Every operation that writes to disk must live here and be tested (see CLAUDE.md).

pub mod entry;
pub mod format;
pub mod icons;
pub mod ops;
pub mod places;
pub mod sort;
pub mod thumbs;
pub mod volume;

pub use entry::{Entry, list_dir};
pub use sort::{SortKey, natural_cmp, sort_entries};
