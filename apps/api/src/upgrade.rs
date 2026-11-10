//! `api engine-export <dir>` and `api engine-verify <dir>`: the two halves of an `ironcalc`
//! upgrade (docs/deployment.md). Run the export with the old binary, deploy, run the verify with
//! the new one.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use sqlx::PgPool;
use tronc::Error;
use uuid::Uuid;

use crate::engine::{self, ENGINE_VERSION};
use crate::modules::workbooks::history;

/// Runs `engine-export` or `engine-verify` when `args[1]` names one, and `None` otherwise.
pub async fn command(args: &[String], pool: &PgPool) -> Option<ExitCode> {
    let verb = args.get(1).map(String::as_str)?;
    if verb != "engine-export" && verb != "engine-verify" {
        return None;
    }
    let Some(dir) = args.get(2).map(PathBuf::from) else {
        eprintln!("usage: api {verb} <backup-dir>");
        return Some(ExitCode::from(2));
    };
    let result = match verb {
        "engine-export" => export_all(pool, &dir).await,
        _ => verify_all(pool, &dir).await,
    };
    Some(result.map_or_else(|e| crate::serve::fail(verb, &e.message), |()| ExitCode::SUCCESS))
}

async fn ids(pool: &PgPool) -> Result<Vec<Uuid>, Error> {
    sqlx::query_scalar("SELECT id FROM workbooks ORDER BY id")
        .fetch_all(pool)
        .await
        .map_err(|e| Error::internal("list workbooks", e))
}

/// Writes a fresh snapshot of every workbook, trashed ones included, and an `.xlsx` copy of
/// each to `<dir>/<id>.xlsx`. Stops at the first failure.
pub async fn export_all(pool: &PgPool, dir: &Path) -> Result<(), Error> {
    std::fs::create_dir_all(dir).map_err(|e| Error::internal("create the backup directory", e))?;
    let ids = ids(pool).await?;
    for id in &ids {
        let loaded = history::load(pool, *id).await?;
        history::snapshot(pool, *id, &loaded.model.to_bytes(), loaded.seq).await?;
        let xlsx = engine::export_xlsx(&loaded.model)?;
        std::fs::write(dir.join(format!("{id}.xlsx")), xlsx).map_err(|e| Error::internal("write a backup", e))?;
    }
    println!("engine-export: {} workbooks exported to {} with ironcalc {ENGINE_VERSION}", ids.len(), dir.display());
    Ok(())
}

/// Loads every workbook with this binary's engine. One that fails is re-imported from
/// `<dir>/<id>.xlsx`. Fails when any workbook could be neither loaded nor re-imported.
pub async fn verify_all(pool: &PgPool, dir: &Path) -> Result<(), Error> {
    let (mut loaded, mut restored, mut lost) = (0, 0, Vec::new());
    for id in ids(pool).await? {
        match history::load(pool, id).await {
            Ok(_) => loaded += 1,
            Err(_) => match reimport(pool, id, dir).await {
                Ok(()) => restored += 1,
                Err(error) => lost.push(format!("{id}: {}", error.message)),
            },
        }
    }
    sqlx::query("UPDATE workbooks SET engine_version = $1")
        .bind(ENGINE_VERSION)
        .execute(pool)
        .await
        .map_err(|e| Error::internal("stamp the engine version", e))?;
    println!("engine-verify: {loaded} loaded, {restored} re-imported from xlsx, {} lost", lost.len());
    match lost.is_empty() {
        true => Ok(()),
        false => Err(Error::internal("engine-verify", lost.join("; "))),
    }
}

async fn reimport(pool: &PgPool, id: Uuid, dir: &Path) -> Result<(), Error> {
    let bytes = std::fs::read(dir.join(format!("{id}.xlsx"))).map_err(|e| Error::internal("read the backup", e))?;
    let (title, locale, timezone, language): (String, String, String, String) =
        sqlx::query_as("SELECT title, locale, timezone, language FROM workbooks WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(|e| Error::internal("read a workbook", e))?;
    let model = engine::import_xlsx(&bytes, &title, &locale, &timezone, &language)?;
    let seq: i64 = sqlx::query_scalar("SELECT seq FROM workbooks WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::internal("read a workbook", e))?;
    history::snapshot(pool, id, &model.to_bytes(), u64::try_from(seq).unwrap_or(0)).await
}
