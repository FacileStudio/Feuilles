use serde::{Deserialize, Serialize};
use tronc::Error;

use super::Model;

/// A sheet named by its 0-based index or by its tab name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum SheetRef {
    /// The position, from 0.
    Index(u32),
    /// The tab name, matched without regard to case.
    Name(String),
}

impl Default for SheetRef {
    fn default() -> Self {
        Self::Index(0)
    }
}

/// The tab names of the model, in order.
pub fn sheet_names(model: &Model) -> Vec<String> {
    model.get_model().workbook.worksheets.iter().map(|sheet| sheet.get_name()).collect()
}

/// The index `sheet` names among `names`, or a 404 that lists the sheets there are.
pub fn sheet_index(names: &[String], sheet: &SheetRef) -> Result<u32, Error> {
    let position = match sheet {
        SheetRef::Index(index) => usize::try_from(*index).ok().filter(|index| *index < names.len()),
        SheetRef::Name(name) => names.iter().position(|known| known.eq_ignore_ascii_case(name.trim())),
    };
    let (wanted, listed) = match sheet {
        SheetRef::Index(index) => {
            (index.to_string(), names.iter().zip(0..).map(|(n, i)| format!("{i} '{n}'")).collect())
        }
        SheetRef::Name(name) => (format!("'{name}'"), names.iter().map(|n| format!("'{n}'")).collect::<Vec<_>>()),
    };
    position
        .and_then(|index| u32::try_from(index).ok())
        .ok_or_else(|| Error::not_found(format!("sheet {wanted} not found; sheets are: {}", listed.join(", "))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sheets_resolve_by_index_or_name_and_a_miss_lists_them() {
        let names = vec!["Budget".to_owned(), "Notes".to_owned()];
        assert_eq!(sheet_index(&names, &SheetRef::Name(" notes ".into())).expect("name"), 1);
        assert_eq!(sheet_index(&names, &SheetRef::Index(0)).expect("index"), 0);
        let miss = sheet_index(&names, &SheetRef::Name("Foo".into())).unwrap_err();
        assert_eq!(miss.message, "sheet 'Foo' not found; sheets are: 'Budget', 'Notes'");
        let miss = sheet_index(&names, &SheetRef::Index(2)).unwrap_err();
        assert_eq!(miss.message, "sheet 2 not found; sheets are: 0 'Budget', 1 'Notes'");
    }
}
