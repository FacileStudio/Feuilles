use std::io::Cursor;

use ironcalc::base::{Model as Inner, UserModel};
use ironcalc::export::save_xlsx_to_writer;
use ironcalc::import::load_from_xlsx_bytes;
use tronc::Error;

use super::{Model, inflate, language};

/// A model from the bytes of an `.xlsx` file: 400 when IronCalc cannot read it, 413 when it
/// inflates past [`inflate::MAX_TOTAL_BYTES`].
pub fn import_xlsx(bytes: &[u8], name: &str, locale: &str, timezone: &str, language_id: &str) -> Result<Model, Error> {
    let language_id = language(language_id)?;
    inflate::check(bytes)?;
    let unreadable = |e: String| Error::invalid(format!("the file is not a workbook IronCalc can read: {e}"));
    let workbook = load_from_xlsx_bytes(bytes, name, locale, timezone).map_err(|e| unreadable(e.to_string()))?;
    let mut model = UserModel::from_model(Inner::from_workbook(workbook, language_id).map_err(unreadable)?);
    model.evaluate();
    Ok(model)
}

/// The model as an `.xlsx` file.
pub fn export_xlsx(model: &Model) -> Result<Vec<u8>, Error> {
    let writer = save_xlsx_to_writer(model.get_model(), Cursor::new(Vec::new()))
        .map_err(|e| Error::internal("export xlsx", e.to_string()))?;
    Ok(writer.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::new_model;

    #[test]
    fn xlsx_round_trips_cells_and_formulas() {
        let mut model = new_model("Book", "en", "UTC", "en").expect("model");
        model.set_user_input(0, 1, 1, "2").expect("input");
        model.set_user_input(0, 2, 1, "=A1*21").expect("input");
        let bytes = export_xlsx(&model).expect("export");
        let again = import_xlsx(&bytes, "Book", "en", "UTC", "en").expect("import");
        assert_eq!(again.get_cell_content(0, 2, 1).expect("content"), "=A1*21");
        assert_eq!(again.get_formatted_cell_value(0, 2, 1).expect("value"), "42");
    }

    #[test]
    fn garbage_is_not_a_workbook() {
        let error = import_xlsx(b"not a zip", "Book", "en", "UTC", "en").unwrap_err();
        assert_eq!(error.code, tronc::Code::InvalidArgument);
    }
}
