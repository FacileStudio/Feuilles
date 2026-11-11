use std::borrow::Cow;
use std::cmp::Reverse;

use ironcalc::base::{Model as Inner, UserModel};
use tronc::Error;

use super::csv_cells::{fill, sheet_name};
use super::{Model, language, new_model};

const DELIMITERS: [u8; 3] = *b",;\t";

/// A one-sheet model from the bytes of a CSV file, the sheet named after `title`.
///
/// The text is UTF-8, with or without a BOM, or else Windows-1252. The delimiter comes from an
/// Excel `sep=` first line, or else is whichever of `,`, `;` and tab the first record holds most
/// (ties go to `,`). Each field is typed as IronCalc types an input, except that formulas and
/// digit identifiers stay text (see `csv_cells`).
pub fn import_csv(bytes: &[u8], title: &str, locale: &str, timezone: &str, language_id: &str) -> Result<Model, Error> {
    let text = decode(bytes);
    let (delimiter, body) = dialect(&text);
    let blank = new_model(title, locale, timezone, language_id)?.to_bytes();
    let mut inner =
        Inner::from_bytes(&blank, language(language_id)?).map_err(|e| Error::internal("start a csv import", e))?;
    fill(&mut inner, body, delimiter)?;
    if let Some(name) = sheet_name(title) {
        inner.rename_sheet_by_index(0, &name).map_err(|e| Error::internal("name the imported sheet", e))?;
    }
    let mut model = UserModel::from_model(inner);
    model.evaluate();
    Ok(model)
}

/// The text of the file: the UTF-8 BOM dropped, Windows-1252 when it is not UTF-8.
pub fn decode(bytes: &[u8]) -> Cow<'_, str> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(text) => Cow::Borrowed(text),
        Err(_) => encoding_rs::WINDOWS_1252.decode_without_bom_handling(bytes).0,
    }
}

/// The delimiter and the records that follow it.
pub fn dialect(text: &str) -> (u8, &str) {
    let (first, rest) = text.split_once('\n').unwrap_or((text, ""));
    match first.trim_end_matches('\r').strip_prefix("sep=").map(str::as_bytes) {
        Some([delimiter @ (b',' | b';' | b'\t' | b'|')]) => (*delimiter, rest),
        _ => (sniff(text), text),
    }
}

fn sniff(text: &str) -> u8 {
    let mut counts = [0usize; DELIMITERS.len()];
    let mut quoted = false;
    for byte in text.bytes() {
        quoted ^= byte == b'"';
        if !quoted && byte == b'\n' {
            break;
        }
        if let Some(index) = DELIMITERS.iter().position(|d| *d == byte).filter(|_| !quoted) {
            counts[index] += 1;
        }
    }
    (0..DELIMITERS.len()).max_by_key(|&i| (counts[i], Reverse(i))).map_or(b',', |i| DELIMITERS[i])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bom_goes_and_latin_bytes_read_as_windows_1252() {
        assert_eq!(decode(b"\xEF\xBB\xBFa;b"), "a;b");
        assert_eq!(decode(b"caf\xE9 \x80"), "café €");
    }

    #[test]
    fn the_delimiter_is_sniffed_outside_quotes_or_read_from_sep() {
        assert_eq!(dialect("a;b;c\n1,5;2;3"), (b';', "a;b;c\n1,5;2;3"));
        assert_eq!(dialect("\"x;y;z\",b\n"), (b',', "\"x;y;z\",b\n"));
        assert_eq!(dialect("a\tb\n"), (b'\t', "a\tb\n"));
        assert_eq!(dialect("single"), (b',', "single"));
        assert_eq!(dialect("sep=;\r\na,b;c"), (b';', "a,b;c"));
    }
}
