//! `api import-xlsx`: an operator command that turns `.xlsx` files into workbooks of a space,
//! through the same engine path as `POST /workbooks/import`, with no session and no owner.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use sqlx::PgPool;
use tronc::Error;

use crate::engine::{self, SheetInfo};
use crate::modules::workbooks::{service, xlsx};

/// Printed on a malformed command line.
pub const USAGE: &str =
    "usage: api import-xlsx --space <registre space id> [--title <title>] [--dry-run] <file.xlsx>...";

const MISSING: &str = "does not exist or was deleted: spaces arrive through the Registre directory sync";

/// The parsed command line.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Args {
    /// The Registre space id the workbooks go to.
    pub space: String,
    /// The title of the one workbook, when given.
    pub title: Option<String>,
    /// Read and report, write nothing.
    pub dry_run: bool,
    /// The files, in order.
    pub files: Vec<PathBuf>,
}

/// What one run did: the lines for stdout and stderr, and the exit code (0, 1 or 2).
#[derive(Debug, Default)]
pub struct Outcome {
    /// One line per workbook created, or per workbook a dry run would create.
    pub out: Vec<String>,
    /// One line per failure.
    pub err: Vec<String>,
    /// 0 ok, 1 a file or the space failed, 2 usage.
    pub code: u8,
}

/// Parses `import-xlsx` arguments after the verb: `Err` holds the usage message.
pub fn parse(rest: &[String]) -> Result<Args, String> {
    let mut args = Args::default();
    let mut iter = rest.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--dry-run" => args.dry_run = true,
            "--space" => args.space = iter.next().cloned().unwrap_or_default(),
            "--title" => args.title = Some(iter.next().cloned().unwrap_or_default()),
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}\n{USAGE}")),
            file => args.files.push(PathBuf::from(file)),
        }
    }
    if let Some(title) = &args.title {
        service::title(title).map_err(|e| format!("--title: {}\n{USAGE}", e.message))?;
    }
    match (args.space.trim().is_empty(), args.files.is_empty(), args.title.is_some() && args.files.len() > 1) {
        (false, false, false) => Ok(args),
        (_, _, true) => Err(format!("--title takes exactly one file\n{USAGE}")),
        _ => Err(USAGE.to_owned()),
    }
}

/// Runs `import-xlsx` when `args[1]` names it, and `None` otherwise.
pub async fn command(args: &[String], pool: &PgPool) -> Option<ExitCode> {
    if args.get(1).map(String::as_str) != Some("import-xlsx") {
        return None;
    }
    let outcome = match parse(&args[2..]) {
        Ok(parsed) => run(pool, &parsed).await,
        Err(usage) => Outcome { err: vec![usage], code: 2, ..Outcome::default() },
    };
    outcome.out.iter().for_each(|line| println!("{line}"));
    outcome.err.iter().for_each(|line| eprintln!("{line}"));
    Some(ExitCode::from(outcome.code))
}

/// Imports every file into the space; a file that fails writes nothing and the others go on.
pub async fn run(pool: &PgPool, args: &Args) -> Outcome {
    if let Some(refused) = refuse_space(pool, &args.space).await {
        return Outcome {
            err: vec![format!("import-xlsx: space {:?} {refused}", args.space)],
            code: 1,
            ..Outcome::default()
        };
    }
    let mut outcome = Outcome::default();
    for file in &args.files {
        match one(pool, args, file).await {
            Ok(line) => outcome.out.push(line),
            Err(error) => {
                outcome.err.push(format!("import-xlsx: {}: {}", file.display(), error.message));
                outcome.code = 1;
            }
        }
    }
    outcome
}

async fn refuse_space(pool: &PgPool, space: &str) -> Option<String> {
    let live = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM spaces WHERE id = $1 AND deleted_at IS NULL)")
        .bind(space)
        .fetch_one(pool)
        .await;
    match live {
        Ok(true) => None,
        Ok(false) => Some(MISSING.to_owned()),
        Err(error) => Some(format!("could not be read: {error}")),
    }
}

async fn one(pool: &PgPool, args: &Args, file: &Path) -> Result<String, Error> {
    let name = file.file_name().and_then(|name| name.to_str()).unwrap_or_default();
    let stem = xlsx::stem(name).ok_or_else(|| Error::invalid("not an .xlsx file"))?;
    let title = args.title.as_deref().map_or_else(|| xlsx::title(stem), xlsx::title);
    let size = std::fs::metadata(file).map_err(|e| Error::invalid(format!("cannot read the file: {e}")))?.len();
    if size > xlsx::MAX_IMPORT_BYTES as u64 {
        return Err(Error::too_large("the file is larger than 20 MiB"));
    }
    let bytes = std::fs::read(file).map_err(|e| Error::invalid(format!("cannot read the file: {e}")))?;
    let model = xlsx::read(&bytes, &title)?;
    let sheets = describe(&engine::sheets(&model));
    if args.dry_run {
        return Ok(format!("would create {title:?} in {}: {sheets}", args.space));
    }
    let row = xlsx::store(pool, Some(&args.space), None, &title, &model).await?;
    Ok(format!("created {} {title:?} in {}: {sheets}", row.id, args.space))
}

fn describe(sheets: &[SheetInfo]) -> String {
    let list: Vec<String> = sheets
        .iter()
        .map(|sheet| format!("{} {}", sheet.name, sheet.used_range.as_deref().unwrap_or("empty")))
        .collect();
    format!("{} sheets ({})", sheets.len(), list.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_line_needs_a_space_and_a_file() {
        let argv = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<String>>();
        let ok = parse(&argv(&["--space", "s", "--dry-run", "a.xlsx", "--title", "Q4"]));
        let want = Args { space: "s".into(), title: Some("Q4".into()), dry_run: true, files: vec!["a.xlsx".into()] };
        assert_eq!(ok, Ok(want));
        assert!(parse(&argv(&["a.xlsx"])).is_err());
        assert!(parse(&argv(&["--space", "s"])).is_err());
        assert!(parse(&argv(&["--space", "s", "--title", "T", "a.xlsx", "b.xlsx"])).is_err());
        assert!(parse(&argv(&["--space", "s", "--title", " ", "a.xlsx"])).is_err());
        assert!(parse(&argv(&["--space", "s", "--force", "a.xlsx"])).is_err());
    }

    #[test]
    fn a_description_names_every_sheet_and_its_used_range() {
        let sheets = [
            SheetInfo { index: 0, name: "Data".into(), used_range: Some("A1:B3".into()) },
            SheetInfo { index: 1, name: "Notes".into(), used_range: None },
        ];
        assert_eq!(describe(&sheets), "2 sheets (Data A1:B3, Notes empty)");
    }
}
