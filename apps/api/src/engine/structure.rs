use ironcalc::base::expressions::types::Area as Span;
use tronc::Error;

use super::sheets::{SheetRef, sheet_index, sheet_names};
use super::{Area, MAX_COLUMN, MAX_ROW, Model};

/// The most rows or columns one insert or delete touches.
pub const MAX_LINES: i32 = 10_000;

/// The most cells of data one clear may cover.
pub const MAX_CLEAR_CELLS: i64 = 1_000_000;

/// Rows or columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Rows, numbered from 1.
    Rows,
    /// Columns, numbered from 1 (`A` is 1).
    Columns,
}

/// What a structural edit does to the sheet it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditKind {
    /// Gives the sheet a new tab name.
    Rename(String),
    /// Moves the sheet to this 0-based position.
    Move(u32),
    /// Deletes the sheet.
    DeleteSheet,
    /// Inserts `count` blank rows or columns before `at`.
    Insert { axis: Axis, at: i32, count: i32 },
    /// Deletes `count` rows or columns from `at`.
    Delete { axis: Axis, at: i32, count: i32 },
    /// Clears the contents of an area, keeping its formatting.
    Clear(Area),
}

/// One structural edit on one sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    /// The sheet it applies to.
    pub sheet: SheetRef,
    /// What it does.
    pub kind: EditKind,
}

/// Resolves the sheet `edit` names among `names` and refuses what [`apply_edit`] would refuse,
/// without a model: an unknown sheet, the last sheet, an empty name, a position or a span out of
/// bounds.
pub fn check_edit(names: &[String], edit: &Edit) -> Result<u32, Error> {
    let sheet = sheet_index(names, &edit.sheet)?;
    match &edit.kind {
        EditKind::Rename(name) => check_name(name).map(drop),
        EditKind::Move(to) => check_position(*to, names.len()).map(drop),
        EditKind::DeleteSheet if names.len() == 1 => {
            Err(Error::failed("a workbook keeps at least one sheet; add another sheet first"))
        }
        EditKind::Insert { axis, at, count } | EditKind::Delete { axis, at, count } => {
            lines(*at, *count, last(*axis)).map(drop)
        }
        EditKind::DeleteSheet | EditKind::Clear(_) => Ok(()),
    }?;
    Ok(sheet)
}

/// Applies `edit`. A refused edit can leave `model` half-written: drop it then.
pub fn apply_edit(model: &mut Model, edit: &Edit) -> Result<(), Error> {
    let sheet = check_edit(&sheet_names(model), edit)?;
    let done = match &edit.kind {
        EditKind::Rename(name) => model.rename_sheet(sheet, name.trim()),
        EditKind::Move(to) => model.move_sheet(sheet, *to),
        EditKind::DeleteSheet => model.delete_sheet(sheet),
        EditKind::Insert { axis: Axis::Rows, at, count } => model.insert_rows(sheet, *at, *count),
        EditKind::Insert { axis: Axis::Columns, at, count } => model.insert_columns(sheet, *at, *count),
        EditKind::Delete { axis: Axis::Rows, at, count } => model.delete_rows(sheet, *at, *count),
        EditKind::Delete { axis: Axis::Columns, at, count } => model.delete_columns(sheet, *at, *count),
        EditKind::Clear(area) => return clear(model, sheet, *area),
    };
    done.map_err(Error::invalid)
}

fn last(axis: Axis) -> i32 {
    match axis {
        Axis::Rows => MAX_ROW,
        Axis::Columns => MAX_COLUMN,
    }
}

fn check_name(name: &str) -> Result<String, Error> {
    match name.trim() {
        "" => Err(Error::invalid("name must not be empty")),
        trimmed => Ok(trimmed.to_owned()),
    }
}

fn check_position(to: u32, sheets: usize) -> Result<u32, Error> {
    match usize::try_from(to).is_ok_and(|to| to < sheets) {
        true => Ok(to),
        false => Err(Error::invalid(format!("position must be 0 to {}", sheets.saturating_sub(1)))),
    }
}

fn lines(at: i32, count: i32, last: i32) -> Result<i32, Error> {
    if !(1..=MAX_LINES).contains(&count) {
        return Err(Error::invalid(format!("count must be 1 to {MAX_LINES}")));
    }
    match (1..=last).contains(&at) && at.saturating_add(count - 1) <= last {
        true => Ok(count),
        false => Err(Error::invalid(format!("at and count must stay within 1 to {last}"))),
    }
}

fn clear(model: &mut Model, sheet: u32, area: Area) -> Result<(), Error> {
    let worksheet = model.get_model().workbook.worksheet(sheet).map_err(Error::invalid)?;
    if worksheet.sheet_data.is_empty() {
        return Ok(());
    }
    let used = worksheet.dimension();
    let top = area.top.max(used.min_row);
    let left = area.left.max(used.min_column);
    let (bottom, right) = (area.bottom.min(used.max_row), area.right.min(used.max_column));
    if top > bottom || left > right {
        return Ok(());
    }
    let span = Area { top, left, bottom, right };
    if span.cells() > MAX_CLEAR_CELLS {
        return Err(Error::invalid(format!(
            "the range covers {} cells of data, more than {MAX_CLEAR_CELLS}; clear a narrower range",
            span.cells()
        )));
    }
    let span = Span { sheet, row: top, column: left, width: right - left + 1, height: bottom - top + 1 };
    model.range_clear_contents(&span).map_err(Error::invalid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{new_model, new_sheet, parse_range, sheets};

    #[test]
    fn edits_replay_on_another_model_and_bad_ones_are_refused() {
        let mut model = new_model("Book", "en", "UTC", "en").expect("model");
        let mut other = new_model("Book", "en", "UTC", "en").expect("model");
        let edit = |sheet: SheetRef, kind| Edit { sheet, kind };
        model.set_user_input(0, 2, 1, "x").expect("input");
        model.set_user_input(0, 3, 2, "y").expect("input");
        new_sheet(&mut model, Some("Notes")).expect("sheet");
        apply_edit(&mut model, &edit(SheetRef::Index(0), EditKind::Insert { axis: Axis::Rows, at: 1, count: 2 }))
            .expect("insert");
        apply_edit(&mut model, &edit(SheetRef::Index(0), EditKind::Delete { axis: Axis::Columns, at: 1, count: 1 }))
            .expect("delete");
        apply_edit(&mut model, &edit(SheetRef::Name("notes".into()), EditKind::Move(0))).expect("move");
        apply_edit(&mut model, &edit(SheetRef::Index(1), EditKind::Rename("Data".into()))).expect("rename");
        apply_edit(
            &mut model,
            &edit(SheetRef::Index(1), EditKind::Clear(parse_range("A1:XFD1048576").expect("range"))),
        )
        .expect("clear");
        other.apply_external_diffs(&model.flush_send_queue()).expect("replay");
        assert_eq!(model.to_bytes(), other.to_bytes());
        let names: Vec<String> = sheets(&model).into_iter().map(|s| s.name).collect();
        assert_eq!(names, ["Notes", "Data"]);
        let lines = EditKind::Insert { axis: Axis::Rows, at: MAX_ROW, count: 2 };
        assert!(apply_edit(&mut model, &edit(SheetRef::Index(0), lines)).is_err());
        apply_edit(&mut model, &edit(SheetRef::Index(0), EditKind::DeleteSheet)).expect("delete sheet");
        let last = apply_edit(&mut model, &edit(SheetRef::Index(0), EditKind::DeleteSheet)).unwrap_err();
        assert_eq!(last.code, tronc::Code::FailedPrecondition);
    }
}
