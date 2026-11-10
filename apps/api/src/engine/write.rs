use serde::Deserialize;
use tronc::Error;

use super::{MAX_COLUMN, MAX_ROW, Model};

/// One cell input to set.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, utoipa::ToSchema, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CellWrite {
    /// The sheet index, from 0.
    pub sheet: u32,
    /// The row, from 1.
    pub row: i32,
    /// The column, from 1.
    pub column: i32,
    /// A value or a formula starting with `=`. Empty clears the cell.
    pub input: String,
}

/// Sets every input and evaluates once. Every address is checked before the first write, but
/// a failure can still leave `model` half-written: drop it then. The batch is the next
/// `flush_send_queue()`.
pub fn set_inputs(model: &mut Model, writes: &[CellWrite]) -> Result<(), Error> {
    let sheets = u32::try_from(model.get_model().workbook.worksheets.len()).unwrap_or(u32::MAX);
    if let Some(bad) = writes.iter().find(|w| w.sheet >= sheets) {
        return Err(Error::not_found(format!("sheet {} does not exist", bad.sheet)));
    }
    let outside = |w: &&CellWrite| !(1..=MAX_ROW).contains(&w.row) || !(1..=MAX_COLUMN).contains(&w.column);
    if let Some(bad) = writes.iter().find(outside) {
        return Err(Error::invalid(format!("row {} column {} is outside the sheet", bad.row, bad.column)));
    }
    model.pause_evaluation();
    let applied = writes.iter().try_for_each(|w| model.set_user_input(w.sheet, w.row, w.column, &w.input));
    model.resume_evaluation();
    applied.map_err(Error::invalid)?;
    model.evaluate();
    Ok(())
}

/// Adds a sheet at the end, named `name` when given, and returns its index.
pub fn new_sheet(model: &mut Model, name: Option<&str>) -> Result<u32, Error> {
    model.new_sheet().map_err(Error::invalid)?;
    let index = u32::try_from(model.get_model().workbook.worksheets.len() - 1).unwrap_or(0);
    if let Some(name) = name.map(str::trim).filter(|name| !name.is_empty()) {
        model.rename_sheet(index, name).map_err(Error::invalid)?;
    }
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::new_model;

    fn write(sheet: u32, row: i32, column: i32, input: &str) -> CellWrite {
        CellWrite { sheet, row, column, input: input.into() }
    }

    #[test]
    fn writes_become_one_batch_another_model_can_replay() {
        let mut model = new_model("Book", "en", "UTC", "en").expect("model");
        let mut other = new_model("Book", "en", "UTC", "en").expect("model");
        set_inputs(&mut model, &[write(0, 1, 1, "20"), write(0, 1, 2, "=A1+1")]).expect("writes");
        other.apply_external_diffs(&model.flush_send_queue()).expect("replay");
        assert_eq!(other.get_formatted_cell_value(0, 1, 2).expect("value"), "21");
        assert_eq!(model.to_bytes(), other.to_bytes());
    }

    #[test]
    fn bad_addresses_are_refused_before_any_write() {
        let mut model = new_model("Book", "en", "UTC", "en").expect("model");
        let before = model.to_bytes();
        assert_eq!(
            set_inputs(&mut model, &[write(0, 1, 1, "1"), write(2, 1, 1, "x")]).unwrap_err().code,
            tronc::Code::NotFound
        );
        assert!(set_inputs(&mut model, &[write(0, 0, 1, "x")]).is_err());
        assert!(set_inputs(&mut model, &[write(0, 1, MAX_COLUMN + 1, "x")]).is_err());
        assert_eq!(model.to_bytes(), before);
    }

    #[test]
    fn a_new_sheet_takes_its_name() {
        let mut model = new_model("Book", "en", "UTC", "en").expect("model");
        assert_eq!(new_sheet(&mut model, Some("Budget")).expect("sheet"), 1);
        assert!(!model.flush_send_queue().is_empty());
        assert_eq!(model.get_model().workbook.worksheets[1].get_name(), "Budget");
    }
}
