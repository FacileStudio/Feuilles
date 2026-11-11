mod common;

use std::path::PathBuf;

use api::import::{Args, run};
use sqlx::PgPool;

fn xlsx(name: &str) -> PathBuf {
    let mut model = ironcalc::base::Model::new_empty("Book", "en", "UTC", "en").expect("model");
    model.set_user_input(0, 1, 1, "7".to_owned()).expect("A1");
    model.set_user_input(0, 2, 2, "=A1*6".to_owned()).expect("B2");
    model.add_sheet("Notes").expect("sheet");
    model.evaluate();
    let bytes = ironcalc::export::save_xlsx_to_writer(&model, std::io::Cursor::new(Vec::new())).expect("xlsx");
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(common::unique("import"));
    std::fs::create_dir_all(&dir).expect("dir");
    let path = dir.join(name);
    std::fs::write(&path, bytes.into_inner()).expect("write");
    path
}

fn args(space: &str, files: Vec<PathBuf>) -> Args {
    Args { space: space.to_owned(), title: None, dry_run: false, files }
}

async fn count(pool: &PgPool, space: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM workbooks WHERE space_id = $1")
        .bind(space)
        .fetch_one(pool)
        .await
        .expect("count")
}

#[tokio::test]
async fn files_become_ownerless_space_workbooks_and_a_bad_file_fails_alone() {
    let Some(state) = common::state().await else { return };
    let space = common::space(&state, &[]).await;
    let good = xlsx("Q4 plan.xlsx");
    let broken = good.with_file_name("broken.xlsx");
    std::fs::write(&broken, b"not a zip").expect("broken");
    let outcome = run(&state.pool, &args(&space, vec![broken, good.clone()])).await;
    assert_eq!((outcome.code, outcome.out.len(), outcome.err.len()), (1, 1, 1), "{outcome:?}");
    assert!(outcome.err[0].contains("broken.xlsx"), "{outcome:?}");
    assert!(outcome.out[0].ends_with("2 sheets (Sheet1 A1:B2, Notes empty)"), "{outcome:?}");
    let id: uuid::Uuid = outcome.out[0].split(' ').nth(1).expect("id").parse().expect("uuid");
    let row = api::modules::workbooks::store::find(&state.pool, id).await.expect("find").expect("row");
    assert_eq!((row.space_id.as_deref(), row.owner_id, row.title.as_str()), (Some(space.as_str()), None, "Q4 plan"));
    assert_eq!(row.engine_version, api::engine::ENGINE_VERSION);
    let loaded = api::modules::workbooks::history::load(&state.pool, id).await.expect("load");
    assert_eq!(loaded.model.get_formatted_cell_value(0, 2, 2).expect("B2"), "42");
    let titled = Args { title: Some(" Budget ".to_owned()), ..args(&space, vec![good]) };
    assert_eq!(run(&state.pool, &titled).await.code, 0);
    let titles: Vec<String> = sqlx::query_scalar("SELECT title FROM workbooks WHERE space_id = $1 ORDER BY title")
        .bind(&space)
        .fetch_all(&state.pool)
        .await
        .expect("titles");
    assert_eq!(titles, ["Budget", "Q4 plan"]);
}

#[tokio::test]
async fn a_missing_or_deleted_space_is_refused_by_name() {
    let Some(state) = common::state().await else { return };
    let gone = common::space(&state, &[]).await;
    sqlx::query("UPDATE spaces SET deleted_at = now() WHERE id = $1")
        .bind(&gone)
        .execute(&state.pool)
        .await
        .expect("drop");
    let unknown = common::unique("space");
    for space in [unknown.as_str(), gone.as_str()] {
        let outcome = run(&state.pool, &args(space, vec![xlsx("book.xlsx")])).await;
        assert_eq!((outcome.code, outcome.out.len()), (1, 0), "{outcome:?}");
        assert!(outcome.err[0].contains(space), "{outcome:?}");
        assert_eq!(count(&state.pool, space).await, 0);
    }
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM spaces WHERE id = $1)")
        .bind(&unknown)
        .fetch_one(&state.pool)
        .await
        .expect("exists");
    assert!(!exists, "the command must never create a space");
}

#[tokio::test]
async fn a_dry_run_reports_and_writes_nothing() {
    let Some(state) = common::state().await else { return };
    let space = common::space(&state, &[]).await;
    let book = xlsx("Forecast.xlsx");
    let csv = book.with_file_name("People.csv");
    std::fs::write(&csv, "name;age\nAda;36\n").expect("csv");
    let dry = Args { dry_run: true, ..args(&space, vec![book, csv]) };
    let outcome = run(&state.pool, &dry).await;
    assert_eq!(outcome.code, 0, "{outcome:?}");
    let want = [
        format!("would create \"Forecast\" in {space}: 2 sheets (Sheet1 A1:B2, Notes empty)"),
        format!("would create \"People\" in {space}: 1 sheets (People A1:B2)"),
    ];
    assert_eq!(outcome.out, want);
    assert_eq!(count(&state.pool, &space).await, 0);
}
