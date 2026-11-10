#![deny(unused, dead_code)]

//! Simulation of the Feuilles collaboration design (plan section 7.1) on IronCalc.

mod client;
mod edit;
mod room;
mod server;

pub use client::Client;
pub use edit::{Axis, Edit, Rect, apply_edit};
pub use room::{Room, Step, run};
pub use server::{Downstream, Server, Stats, Update};

use ironcalc::base::UserModel;

/// Language every model in the spike uses.
pub const LANGUAGE: &str = "en";

/// Height of a full column, as IronCalc expects it for column styles.
pub const FULL_COLUMN_HEIGHT: i32 = 1_048_576;

/// Creates the empty workbook every replica starts from.
pub fn new_model() -> UserModel<'static> {
    UserModel::new_empty("Book", "en", "UTC", LANGUAGE).expect("valid locale")
}

/// Rebuilds a model from bytes.
pub fn from_bytes(bytes: &[u8]) -> UserModel<'static> {
    UserModel::from_bytes(bytes, LANGUAGE).expect("valid snapshot")
}
