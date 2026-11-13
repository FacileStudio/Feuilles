//! The IronCalc engine, behind the few calls the API makes on it.

mod csv_cells;
mod csv_export;
mod csv_import;
mod files;
mod inflate;
mod range;
mod read;
mod sheets;
mod structure;
mod write;

pub use csv_export::export_csv;
pub use csv_import::import_csv;
pub use files::{export_xlsx, import_xlsx};
pub use range::{Area, MAX_COLUMN, MAX_ROW, column_name, parse_range};
pub use read::{Cell, SheetInfo, cell_count, read_area, sheet_count, sheets};
pub use sheets::{SheetRef, sheet_index, sheet_names};
pub use structure::{Axis, Edit, EditKind, MAX_CLEAR_CELLS, MAX_LINES, apply_edit, check_edit};
pub use write::{CellWrite, new_sheet, set_inputs};

use ironcalc::base::UserModel;
use tronc::Error;

/// A workbook model that owns everything it borrows.
pub type Model = UserModel<'static>;

/// The `ironcalc` crate version the snapshots and diffs are written with. The browser's
/// `@ironcalc/wasm` must be this exact version: the diff format is private to the crate.
pub const ENGINE_VERSION: &str = "0.9.1";

/// The function-name languages this build accepts for a workbook.
pub const LANGUAGES: [&str; 5] = ["en", "es", "fr", "de", "it"];

/// The `'static` spelling of a supported language, or 400.
pub fn language(id: &str) -> Result<&'static str, Error> {
    LANGUAGES
        .iter()
        .copied()
        .find(|known| *known == id)
        .ok_or_else(|| Error::invalid(format!("language must be one of {}", LANGUAGES.join(", "))))
}

/// A new workbook with one empty sheet.
pub fn new_model(title: &str, locale: &str, timezone: &str, language_id: &str) -> Result<Model, Error> {
    let language_id = language(language_id)?;
    let model = UserModel::new_empty(title, locale, timezone, language_id).map_err(Error::invalid)?;
    load(&model.to_bytes(), language_id)
}

/// A model from its snapshot bytes.
pub fn load(bytes: &[u8], language_id: &str) -> Result<Model, Error> {
    let language_id = language(language_id)?;
    UserModel::from_bytes(bytes, language_id).map_err(|e| Error::internal("load workbook snapshot", e))
}

/// The bytes a flush with nothing in the queue produces. A batch equal to it changes nothing.
pub fn empty_batch() -> Vec<u8> {
    UserModel::new_empty("empty", "en", "UTC", "en").map(|mut model| model.flush_send_queue()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_language_loads_and_others_are_refused() {
        for id in LANGUAGES {
            new_model("Book", "en", "UTC", id).unwrap_or_else(|e| panic!("{id}: {}", e.message));
        }
        assert_eq!(language("xx").unwrap_err().code, tronc::Code::InvalidArgument);
    }

    #[test]
    fn a_snapshot_round_trips_with_its_cells() {
        let mut model = new_model("Book", "en", "UTC", "en").expect("model");
        model.set_user_input(0, 1, 1, "=1+2").expect("input");
        let again = load(&model.to_bytes(), "en").expect("load");
        assert_eq!(again.get_formatted_cell_value(0, 1, 1).expect("value"), "3");
    }

    #[test]
    fn the_engine_pin_matches_the_manifest() {
        let manifest = include_str!("../../Cargo.toml");
        assert!(manifest.contains(&format!("ironcalc = \"={ENGINE_VERSION}\"")));
    }
}
