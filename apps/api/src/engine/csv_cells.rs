use std::borrow::Cow;

use csv::{ReaderBuilder, StringRecord};
use ironcalc::base::Model as Inner;
use tronc::Error;

use super::csv_export::MAX_CSV_CELLS;
use super::{MAX_COLUMN, MAX_ROW};

/// Writes every record into sheet 0 from `A1`: 413 past the engine's rows or columns or past
/// [`MAX_CSV_CELLS`] filled cells.
pub fn fill(model: &mut Inner, body: &str, delimiter: u8) -> Result<(), Error> {
    let mut reader =
        ReaderBuilder::new().delimiter(delimiter).has_headers(false).flexible(true).from_reader(body.as_bytes());
    let mut cells = 0;
    for (index, record) in reader.records().enumerate() {
        let record = record.map_err(|e| Error::invalid(format!("the CSV cannot be read: {e}")))?;
        let row = i32::try_from(index + 1).ok().filter(|row| *row <= MAX_ROW);
        let row = row.ok_or_else(|| Error::too_large(format!("the CSV has more than {MAX_ROW} rows")))?;
        if record.len() > MAX_COLUMN as usize {
            return Err(Error::too_large(format!("row {row} has more than {MAX_COLUMN} columns")));
        }
        cells += record.iter().filter(|field| !field.is_empty()).count() as i64;
        if cells > MAX_CSV_CELLS {
            return Err(Error::too_large(format!("the CSV has more than {MAX_CSV_CELLS} filled cells")));
        }
        write_row(model, row, &record, delimiter == b';')?;
    }
    Ok(())
}

fn write_row(model: &mut Inner, row: i32, record: &StringRecord, decimal_comma: bool) -> Result<(), Error> {
    let fail = |e: String| Error::invalid(format!("row {row}: {e}"));
    for (index, field) in record.iter().enumerate().filter(|(_, field)| !field.is_empty()) {
        let column = index as i32 + 1;
        model.set_user_input(0, row, column, input(field, decimal_comma).into_owned()).map_err(fail)?;
        if model.get_cell_formula(0, row, column).map_err(fail)?.is_some() {
            model.set_user_input(0, row, column, format!("'{field}")).map_err(fail)?;
        }
    }
    Ok(())
}

/// What a field is typed as. `=…` and digit identifiers become quote-prefixed text, a `;` file's
/// decimal comma becomes a point, and everything else goes to IronCalc as typed input.
pub fn input(field: &str, decimal_comma: bool) -> Cow<'_, str> {
    if field.starts_with('=') || identifier(field) {
        return Cow::Owned(format!("'{field}"));
    }
    match decimal_comma.then(|| comma_number(field)).flatten() {
        Some(number) => Cow::Owned(number),
        None => Cow::Borrowed(field),
    }
}

fn identifier(field: &str) -> bool {
    let (plus, digits) = field.strip_prefix('+').map_or((false, field), |rest| (true, rest));
    let count = digits.bytes().filter(u8::is_ascii_digit).count();
    let numeric = count > 0 && digits.bytes().all(|b| b.is_ascii_digit() || b == b' ');
    numeric && ((plus && count > 6) || (!plus && count > 1 && digits.starts_with('0')) || count > 15)
}

fn comma_number(field: &str) -> Option<String> {
    let (whole, fraction) = field.strip_prefix('-').unwrap_or(field).split_once(',')?;
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    (digits(whole) && digits(fraction)).then(|| field.replacen(',', ".", 1))
}

/// A valid sheet name from a title: `\ / * ? : [ ] '` become spaces, runs of space one, 31
/// characters at most.
pub fn sheet_name(title: &str) -> Option<String> {
    let spaced: String = title.chars().map(|c| if "\\/*?:[]'".contains(c) { ' ' } else { c }).collect();
    let clean: String = spaced.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(31).collect();
    let clean = clean.trim_end();
    (!clean.is_empty()).then(|| clean.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_and_formulas_stay_text_and_decimal_commas_become_points() {
        let typed = |field: &str, comma: bool| input(field, comma).into_owned();
        assert_eq!(typed("007", false), "'007");
        assert_eq!(typed("06 12 34 56 78", false), "'06 12 34 56 78");
        assert_eq!(typed("+33612345678", false), "'+33612345678");
        assert_eq!(typed("1234567890123456", false), "'1234567890123456");
        assert_eq!(typed("=1+1", false), "'=1+1");
        assert_eq!(typed("0", false), "0");
        assert_eq!(typed("-12", false), "-12");
        assert_eq!(typed("1,5", true), "1.5");
        assert_eq!(typed("-0,25", true), "-0.25");
        assert_eq!(typed("1,5", false), "1,5");
        assert_eq!(typed("a,b", true), "a,b");
    }

    #[test]
    fn a_sheet_name_drops_what_excel_refuses() {
        assert_eq!(sheet_name("Q4 [draft]: a/b").as_deref(), Some("Q4 draft a b"));
        assert_eq!(sheet_name("Bob's 'list'").as_deref(), Some("Bob s list"));
        assert_eq!(sheet_name("x".repeat(40).as_str()).map(|n| n.len()), Some(31));
        assert_eq!(sheet_name("[]"), None);
    }
}
