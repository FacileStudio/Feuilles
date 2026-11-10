use std::io::Cursor;

use ironcalc::base::{Model as Inner, UserModel};
use ironcalc::export::save_xlsx_to_writer;
use ironcalc::import::load_from_xlsx_bytes;
use tronc::Error;

use super::{Model, language};

/// The most cells a CSV export walks, counting the empty ones up to the last filled cell.
pub const MAX_CSV_CELLS: i64 = 5_000_000;

/// A model from the bytes of an `.xlsx` file, or 400 when IronCalc cannot read it.
pub fn import_xlsx(bytes: &[u8], name: &str, locale: &str, timezone: &str, language_id: &str) -> Result<Model, Error> {
    let language_id = language(language_id)?;
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

/// One sheet as RFC 4180 CSV of the displayed values, from `A1` to the last filled cell.
pub fn export_csv(model: &Model, sheet: u32) -> Result<String, Error> {
    let worksheet = model
        .get_model()
        .workbook
        .worksheet(sheet)
        .map_err(|_| Error::not_found(format!("sheet {sheet} does not exist")))?;
    if worksheet.sheet_data.is_empty() {
        return Ok(String::new());
    }
    let size = worksheet.dimension();
    if i64::from(size.max_row) * i64::from(size.max_column) > MAX_CSV_CELLS {
        return Err(Error::too_large(format!("the sheet spans more than {MAX_CSV_CELLS} cells")));
    }
    let mut out = String::new();
    for row in 1..=size.max_row {
        let fields: Result<Vec<String>, Error> = (1..=size.max_column)
            .map(|column| model.get_formatted_cell_value(sheet, row, column).map(|text| quote(&text)))
            .map(|field| field.map_err(|e| Error::internal("export csv", e)))
            .collect();
        out.push_str(&fields?.join(","));
        out.push_str("\r\n");
    }
    Ok(out)
}

fn quote(field: &str) -> String {
    if field.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_owned()
    }
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

    #[test]
    fn csv_quotes_what_needs_quoting_and_pads_from_a1() {
        let mut model = new_model("Book", "en", "UTC", "en").expect("model");
        model.set_user_input(0, 1, 2, "a,b").expect("input");
        model.set_user_input(0, 2, 1, "say \"hi\"").expect("input");
        assert_eq!(export_csv(&model, 0).expect("csv"), ",\"a,b\"\r\n\"say \"\"hi\"\"\",\r\n");
        assert_eq!(export_csv(&new_model("B", "en", "UTC", "en").expect("model"), 0).expect("csv"), "");
        assert_eq!(export_csv(&model, 9).unwrap_err().code, tronc::Code::NotFound);
    }
}
