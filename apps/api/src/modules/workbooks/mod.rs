//! Workbooks: the list, the files, the cells, and the access rules over all of them.

pub mod actions;
mod cells;
mod files;
mod handlers;
pub mod history;
pub mod model;
mod router;
pub mod service;
pub mod store;
pub mod types;
pub mod xlsx;

pub use router::routes;
pub use xlsx::MAX_IMPORT_BYTES;
