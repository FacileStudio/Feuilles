use std::io::{Cursor, Read};

use tronc::Error;

/// The most entries an imported `.xlsx` may hold.
pub const MAX_ENTRIES: usize = 10_000;
/// The most bytes one entry of an imported `.xlsx` may inflate to.
pub const MAX_ENTRY_BYTES: u64 = 128 << 20;
/// The most bytes all the entries of an imported `.xlsx` may inflate to together.
pub const MAX_TOTAL_BYTES: u64 = 256 << 20;

/// Inflates every entry of a zip archive into nothing, counting the bytes it really produces (the
/// sizes the archive declares can lie): 413 past [`MAX_ENTRIES`], [`MAX_ENTRY_BYTES`] or
/// [`MAX_TOTAL_BYTES`], 400 when it is not a zip. IronCalc reads the same archive afterwards, so
/// it never inflates more than this measured.
pub fn check(bytes: &[u8]) -> Result<(), Error> {
    let unreadable =
        |e: zip::result::ZipError| Error::invalid(format!("the file is not a workbook IronCalc can read: {e}"));
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(unreadable)?;
    if archive.len() > MAX_ENTRIES {
        return Err(Error::too_large(format!("the file holds more than {MAX_ENTRIES} parts")));
    }
    let mut total = 0u64;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(unreadable)?;
        let budget = MAX_ENTRY_BYTES.min(MAX_TOTAL_BYTES - total) + 1;
        let inflated = std::io::copy(&mut entry.take(budget), &mut std::io::sink())
            .map_err(|e| Error::invalid(format!("the file is not a workbook IronCalc can read: {e}")))?;
        total += inflated;
        if inflated >= budget {
            return Err(Error::too_large("the file inflates past 256 MiB, or one part past 128 MiB"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn zip_of(parts: &[(&str, u64)]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, size) in parts {
            writer.start_file(*name, options).expect("start");
            let chunk = vec![b'a'; 1 << 20];
            (0..size / (1 << 20)).for_each(|_| writer.write_all(&chunk).expect("write"));
        }
        writer.finish().expect("finish").into_inner()
    }

    #[test]
    fn a_bomb_is_refused_before_it_inflates_and_a_small_archive_passes() {
        let bomb = zip_of(&[("xl/sharedStrings.xml", 200 << 20)]);
        assert!(bomb.len() < 1 << 20, "the test bomb should be small: {}", bomb.len());
        assert_eq!(check(&bomb).unwrap_err().code, tronc::Code::ResourceExhausted);
        let spread = zip_of(&[("a.xml", 100 << 20), ("b.xml", 100 << 20), ("c.xml", 100 << 20)]);
        assert_eq!(check(&spread).unwrap_err().code, tronc::Code::ResourceExhausted);
        assert!(check(&zip_of(&[("a.xml", 2 << 20), ("b.xml", 0)])).is_ok());
        assert_eq!(check(b"not a zip").unwrap_err().code, tronc::Code::InvalidArgument);
    }
}
