use ironcalc::base::types::CellType;
use tronc::Error;

use super::Model;

/// The most cells a CSV export walks or an import writes, counting the empty ones up to the last
/// filled cell on export.
pub const MAX_CSV_CELLS: i64 = 5_000_000;

/// First characters a spreadsheet may run as a formula, plus the quote that hides them, so a text
/// cell starting with one leaves with a `'` in front (OWASP "CSV injection").
const GUARDED: [char; 7] = ['=', '+', '-', '@', '\t', '\r', '\''];

/// One sheet as RFC 4180 CSV of the displayed values, from `A1` to the last filled cell. A text
/// cell that starts with a [`GUARDED`] character gets a leading `'`; numbers never do.
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
        let fields: Result<Vec<String>, String> =
            (1..=size.max_column).map(|column| field(model, sheet, row, column)).collect();
        out.push_str(&fields.map_err(|e| Error::internal("export csv", e))?.join(","));
        out.push_str("\r\n");
    }
    Ok(out)
}

fn field(model: &Model, sheet: u32, row: i32, column: i32) -> Result<String, String> {
    let text = model.get_formatted_cell_value(sheet, row, column)?;
    let number = matches!(model.get_cell_type(sheet, row, column)?, CellType::Number);
    match !number && text.starts_with(GUARDED) {
        true => Ok(quote(&format!("'{text}"))),
        false => Ok(quote(&text)),
    }
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
    fn csv_quotes_what_needs_quoting_and_pads_from_a1() {
        let mut model = new_model("Book", "en", "UTC", "en").expect("model");
        model.set_user_input(0, 1, 2, "a,b").expect("input");
        model.set_user_input(0, 2, 1, "say \"hi\"").expect("input");
        assert_eq!(export_csv(&model, 0).expect("csv"), ",\"a,b\"\r\n\"say \"\"hi\"\"\",\r\n");
        assert_eq!(export_csv(&new_model("B", "en", "UTC", "en").expect("model"), 0).expect("csv"), "");
        assert_eq!(export_csv(&model, 9).unwrap_err().code, tronc::Code::NotFound);
    }

    #[test]
    fn text_that_a_spreadsheet_would_run_leaves_behind_a_quote() {
        let mut model = new_model("Book", "en", "UTC", "en").expect("model");
        for (column, input) in
            ["'=HYPERLINK(\"x\")", "'+cmd", "'-2+3", "@SUM(1)", "-5", "=-1-1", "''a"].iter().enumerate()
        {
            model.set_user_input(0, 1, column as i32 + 1, input).expect("input");
        }
        let want = "\"'=HYPERLINK(\"\"x\"\")\",'+cmd,'-2+3,'@SUM(1),-5,-2,''a\r\n";
        assert_eq!(export_csv(&model, 0).expect("csv"), want);
    }
}
