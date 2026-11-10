use tronc::Error;

/// The last row IronCalc addresses.
pub const MAX_ROW: i32 = 1_048_576;
/// The last column IronCalc addresses (`XFD`).
pub const MAX_COLUMN: i32 = 16_384;

/// A rectangle of cells, 1-based and inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    /// First row.
    pub top: i32,
    /// First column.
    pub left: i32,
    /// Last row.
    pub bottom: i32,
    /// Last column.
    pub right: i32,
}

impl Area {
    /// How many cells it holds.
    pub fn cells(&self) -> i64 {
        i64::from(self.bottom - self.top + 1) * i64::from(self.right - self.left + 1)
    }

    /// Its `A1:D20` spelling.
    pub fn label(&self) -> String {
        format!("{}{}:{}{}", column_name(self.left), self.top, column_name(self.right), self.bottom)
    }
}

/// Parses `A1` or `A1:D20`, in either corner order, or answers 400.
pub fn parse_range(text: &str) -> Result<Area, Error> {
    let invalid = || Error::invalid(format!("range {text:?} is not of the form A1:D20"));
    let (first, second) = text.trim().split_once(':').unwrap_or((text.trim(), text.trim()));
    let (r1, c1) = parse_cell(first).ok_or_else(invalid)?;
    let (r2, c2) = parse_cell(second).ok_or_else(invalid)?;
    Ok(Area { top: r1.min(r2), left: c1.min(c2), bottom: r1.max(r2), right: c1.max(c2) })
}

fn parse_cell(text: &str) -> Option<(i32, i32)> {
    let split = text.find(|c: char| c.is_ascii_digit())?;
    let (letters, digits) = text.split_at(split);
    if letters.is_empty() || letters.len() > 3 || !letters.bytes().all(|b| b.is_ascii_alphabetic()) {
        return None;
    }
    let column = letters.bytes().fold(0i32, |acc, b| acc * 26 + i32::from(b.to_ascii_uppercase() - b'A' + 1));
    let row: i32 = digits.parse().ok()?;
    ((1..=MAX_ROW).contains(&row) && (1..=MAX_COLUMN).contains(&column)).then_some((row, column))
}

/// The letters of a 1-based column: 1 is `A`, 27 is `AA`.
pub fn column_name(column: i32) -> String {
    let mut letters = Vec::new();
    let mut rest = column;
    while rest > 0 {
        let digit = u8::try_from((rest - 1) % 26).unwrap_or(0);
        letters.push(char::from(b'A' + digit));
        rest = (rest - 1) / 26;
    }
    letters.iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_parse_in_any_corner_order() {
        let area = parse_range("d20:A1").expect("range");
        assert_eq!(area, Area { top: 1, left: 1, bottom: 20, right: 4 });
        assert_eq!(area.label(), "A1:D20");
        assert_eq!(area.cells(), 80);
        assert_eq!(parse_range("B3").expect("cell").label(), "B3:B3");
    }

    #[test]
    fn bad_ranges_are_refused() {
        for text in ["", "A", "1", "A0", "XFE1", "A1048577", "A1:", "ABCD1", "A-1"] {
            assert!(parse_range(text).is_err(), "{text:?} parsed");
        }
    }

    #[test]
    fn column_names_round_trip() {
        for (column, name) in [(1, "A"), (26, "Z"), (27, "AA"), (702, "ZZ"), (703, "AAA"), (MAX_COLUMN, "XFD")] {
            assert_eq!(column_name(column), name);
            assert_eq!(parse_range(&format!("{name}1")).expect("range").left, column);
        }
    }
}
