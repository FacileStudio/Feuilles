use ironcalc::base::cell::CellValue;
use serde::Serialize;
use tronc::Error;

use super::{Area, Model};

/// One cell as the API reads it.
#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema, schemars::JsonSchema)]
pub struct Cell {
    /// What a person typed: a value or a formula starting with `=`.
    pub input: String,
    /// The computed value: a number, a string, a boolean, or null when empty.
    #[schema(value_type = Object)]
    pub value: serde_json::Value,
    /// The value as the grid displays it, with its number format.
    pub formatted: String,
}

/// One sheet of a workbook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema, schemars::JsonSchema)]
pub struct SheetInfo {
    /// Position, from 0. Cell addresses use it.
    pub index: u32,
    /// Its tab name.
    pub name: String,
    /// The smallest range holding every filled cell, or null when the sheet is empty.
    pub used_range: Option<String>,
}

/// Every sheet of the model, in tab order.
pub fn sheets(model: &Model) -> Vec<SheetInfo> {
    let workbook = &model.get_model().workbook;
    workbook
        .worksheets
        .iter()
        .zip(0u32..)
        .map(|(sheet, index)| {
            let used = (!sheet.sheet_data.is_empty()).then(|| sheet.dimension());
            let used_range = used
                .map(|d| Area { top: d.min_row, left: d.min_column, bottom: d.max_row, right: d.max_column }.label());
            SheetInfo { index, name: sheet.get_name(), used_range }
        })
        .collect()
}

/// The cells of `area` on `sheet`, row by row, or 404 for a sheet that does not exist.
pub fn read_area(model: &Model, sheet: u32, area: Area) -> Result<Vec<Vec<Cell>>, Error> {
    let inner = model.get_model();
    inner.workbook.worksheet(sheet).map_err(|_| Error::not_found(format!("sheet {sheet} does not exist")))?;
    (area.top..=area.bottom)
        .map(|row| (area.left..=area.right).map(|column| cell(model, sheet, row, column)).collect())
        .collect()
}

fn cell(model: &Model, sheet: u32, row: i32, column: i32) -> Result<Cell, Error> {
    let fail = |e: String| Error::internal("read cell", e);
    let inner = model.get_model();
    let value = match inner.get_cell_value_by_index(sheet, row, column).map_err(fail)? {
        CellValue::None => serde_json::Value::Null,
        CellValue::String(text) => serde_json::Value::String(text),
        CellValue::Number(number) => serde_json::Number::from_f64(number).map_or(serde_json::Value::Null, Into::into),
        CellValue::Boolean(flag) => serde_json::Value::Bool(flag),
    };
    Ok(Cell {
        input: model.get_cell_content(sheet, row, column).map_err(fail)?,
        value,
        formatted: model.get_formatted_cell_value(sheet, row, column).map_err(fail)?,
    })
}

/// How many sheets the model holds.
pub fn sheet_count(model: &Model) -> usize {
    model.get_model().workbook.worksheets.len()
}

/// How many filled cells the model holds. It walks every cell.
pub fn cell_count(model: &Model) -> usize {
    model.get_model().workbook.worksheets.iter().map(|sheet| sheet.sheet_data.cells().count()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{new_model, parse_range};

    #[test]
    fn reads_inputs_values_and_the_used_range() {
        let mut model = new_model("Book", "en", "UTC", "en").expect("model");
        model.set_user_input(0, 2, 2, "4").expect("input");
        model.set_user_input(0, 3, 3, "=B2*2").expect("input");
        model.set_user_input(0, 1, 1, "TRUE").expect("input");
        let rows = read_area(&model, 0, parse_range("B2:C3").expect("range")).expect("read");
        assert_eq!(rows[0][0].value, serde_json::json!(4.0));
        assert_eq!(rows[1][1].input, "=B2*2");
        assert_eq!(rows[1][1].formatted, "8");
        assert_eq!(rows[0][1].value, serde_json::Value::Null);
        assert_eq!(sheets(&model)[0].used_range.as_deref(), Some("A1:C3"));
        assert_eq!(read_area(&model, 0, parse_range("A1").expect("range")).expect("read")[0][0].value, true);
    }

    #[test]
    fn a_missing_sheet_is_not_found() {
        let model = new_model("Book", "en", "UTC", "en").expect("model");
        let error = read_area(&model, 3, parse_range("A1").expect("range")).unwrap_err();
        assert_eq!(error.code, tronc::Code::NotFound);
        assert_eq!(sheets(&model)[0].used_range, None);
    }
}
