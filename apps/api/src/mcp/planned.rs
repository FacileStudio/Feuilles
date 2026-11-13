use serde::{Deserialize, Serialize};
use tronc::Error;
use uuid::Uuid;

use crate::engine::{Axis, Edit, EditKind, SheetRef, column_name, parse_range};

/// What a destructive edit removes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    /// The whole sheet.
    DeleteSheet,
    /// `count` rows from `at`.
    DeleteRows {
        /// The first row, from 1.
        at: i32,
        /// How many.
        count: i32,
    },
    /// `count` columns from `at`.
    DeleteColumns {
        /// The first column, from 1.
        at: i32,
        /// How many.
        count: i32,
    },
    /// The contents of a range.
    Clear {
        /// Such as `A1:D20`.
        range: String,
    },
}

/// A destructive edit waiting for the person, stored as the pending action's target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Planned {
    /// The workbook.
    pub workbook_id: Uuid,
    /// The sheet, as the agent named it.
    pub sheet: SheetRef,
    /// What goes.
    pub op: Op,
}

impl Planned {
    /// The engine edit, once the range parses.
    pub fn edit(&self) -> Result<Edit, Error> {
        let kind = match &self.op {
            Op::DeleteSheet => EditKind::DeleteSheet,
            Op::DeleteRows { at, count } => EditKind::Delete { axis: Axis::Rows, at: *at, count: *count },
            Op::DeleteColumns { at, count } => EditKind::Delete { axis: Axis::Columns, at: *at, count: *count },
            Op::Clear { range } => EditKind::Clear(parse_range(range)?),
        };
        Ok(Edit { sheet: self.sheet.clone(), kind })
    }

    /// What the person is asked, with the workbook title and the resolved sheet name.
    pub fn question(&self, title: &str, sheet: &str) -> String {
        let span = |noun: &str, first: String, last: String| match first == last {
            true => format!("{noun} {first}"),
            false => format!("{noun}s {first} to {last}"),
        };
        let what = match &self.op {
            Op::DeleteSheet => return format!("Delete the sheet '{sheet}' of \"{title}\" with all its cells?"),
            Op::DeleteRows { at, count } => span("row", at.to_string(), (at + count - 1).to_string()),
            Op::DeleteColumns { at, count } => span("column", column_name(*at), column_name(at + count - 1)),
            Op::Clear { range } => return format!("Clear the cells of {} on '{sheet}' in \"{title}\"?", range.trim()),
        };
        format!("Delete {what} of '{sheet}' in \"{title}\" with their cells?")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn questions_name_what_goes_and_targets_round_trip() {
        let plan = |op| Planned { workbook_id: Uuid::nil(), sheet: SheetRef::Name("Data".into()), op };
        let rows = plan(Op::DeleteRows { at: 3, count: 2 });
        assert_eq!(rows.question("Budget", "Data"), "Delete rows 3 to 4 of 'Data' in \"Budget\" with their cells?");
        let columns = plan(Op::DeleteColumns { at: 26, count: 1 });
        assert_eq!(columns.question("B", "Data"), "Delete column Z of 'Data' in \"B\" with their cells?");
        let stored: Planned = serde_json::from_str(&serde_json::to_string(&rows).expect("json")).expect("parse");
        assert_eq!(stored.edit().expect("edit"), rows.edit().expect("edit"));
        assert!(plan(Op::Clear { range: "nope".into() }).edit().is_err());
    }
}
