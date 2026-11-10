use ironcalc::base::{UserModel, expressions::types::Area};

/// A cell area whose `sheet` is a pick.
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    /// Sheet pick.
    pub sheet: u32,
    /// Top row, 1-based.
    pub row: i32,
    /// Left column, 1-based.
    pub column: i32,
    /// Width in columns.
    pub width: i32,
    /// Height in rows.
    pub height: i32,
}

impl Rect {
    fn area(&self, sheets: u32) -> Area {
        Area {
            sheet: self.sheet % sheets,
            row: self.row,
            column: self.column,
            width: self.width,
            height: self.height,
        }
    }
}

/// What a structural edit inserts or deletes.
#[derive(Debug, Clone, Copy)]
pub enum Axis {
    /// `insert_rows`.
    InsertRows,
    /// `delete_rows`.
    DeleteRows,
    /// `insert_columns`.
    InsertColumns,
    /// `delete_columns`.
    DeleteColumns,
}

/// One user action on a model. Every `sheet` is a pick, taken modulo the sheet count.
#[derive(Debug, Clone)]
pub enum Edit {
    /// `set_user_input` on one cell.
    Input {
        /// Sheet pick.
        sheet: u32,
        /// Row, 1-based.
        row: i32,
        /// Column, 1-based.
        column: i32,
        /// Raw user input.
        value: String,
    },
    /// `update_range_style` on an area.
    Style {
        /// Target area.
        rect: Rect,
        /// Style path, for example `font.b`.
        path: String,
        /// Style value.
        value: String,
    },
    /// A row or column insert or delete.
    Structure {
        /// Sheet pick.
        sheet: u32,
        /// Which operation.
        axis: Axis,
        /// First row or column.
        at: i32,
        /// How many.
        count: i32,
    },
    /// `new_sheet`.
    NewSheet,
    /// `rename_sheet`.
    RenameSheet {
        /// Sheet pick.
        sheet: u32,
        /// New name.
        name: String,
    },
}

/// Applies an edit through the public `UserModel` API, as the browser grid would.
pub fn apply_edit(model: &mut UserModel<'static>, edit: &Edit) -> Result<(), String> {
    let sheets = model.get_worksheets_properties().len() as u32;
    match edit {
        Edit::Input {
            sheet,
            row,
            column,
            value,
        } => model.set_user_input(sheet % sheets, *row, *column, value),
        Edit::Style { rect, path, value } => {
            model.update_range_style(&rect.area(sheets), path, value)
        }
        Edit::Structure {
            sheet,
            axis,
            at,
            count,
        } => apply_structure(model, sheet % sheets, *axis, *at, *count),
        Edit::NewSheet => model.new_sheet(),
        Edit::RenameSheet { sheet, name } => model.rename_sheet(sheet % sheets, name),
    }
}

fn apply_structure(
    model: &mut UserModel<'static>,
    sheet: u32,
    axis: Axis,
    at: i32,
    count: i32,
) -> Result<(), String> {
    match axis {
        Axis::InsertRows => model.insert_rows(sheet, at, count),
        Axis::DeleteRows => model.delete_rows(sheet, at, count),
        Axis::InsertColumns => model.insert_columns(sheet, at, count),
        Axis::DeleteColumns => model.delete_columns(sheet, at, count),
    }
}
