mod common;

use api::upgrade::{export_all, verify_all};
use common::web::{call, create};
use common::{app, person};
use http::Method;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn export_then_verify_restores_a_workbook_whose_snapshot_no_longer_loads() {
    let Some(app) = app().await else { return };
    let pool = &app.state.pool;
    sqlx::query("DELETE FROM workbooks").execute(pool).await.expect("reset");
    let me = person(&app.state).await;
    let id = create(&app.router, &me, None).await["id"].as_str().expect("id").to_owned();
    let writes = json!({"writes": [{"sheet": 0, "row": 1, "column": 1, "input": "=20+22"}]});
    call(&app.router, Method::PUT, &format!("/api/workbooks/{id}/cells"), Some(&me.token), Some(writes)).await;
    let hub = app.state.hub.clone();
    tokio::task::spawn_blocking(move || hub.shutdown()).await.expect("stop the rooms");
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(common::unique("engine"));
    export_all(pool, &dir).await.expect("export");
    assert!(dir.join(format!("{id}.xlsx")).is_file());
    assert_eq!(
        text(pool, "SELECT seq::text || '/' || snapshot_seq FROM workbooks WHERE id = $1::uuid", &id).await,
        "1/1"
    );
    text(pool, "UPDATE workbooks SET snapshot = 'x', engine_version = 'old' WHERE id = $1::uuid RETURNING ''", &id)
        .await;
    verify_all(pool, &dir).await.expect("verify");
    let loaded = api::modules::workbooks::history::load(pool, id.parse().expect("uuid")).await.expect("load");
    assert_eq!(loaded.model.get_formatted_cell_value(0, 1, 1).expect("value"), "42");
    let version = text(pool, "SELECT engine_version FROM workbooks WHERE id = $1::uuid", &id).await;
    assert_eq!(version, api::engine::ENGINE_VERSION);
    std::fs::remove_file(dir.join(format!("{id}.xlsx"))).expect("remove");
    text(pool, "UPDATE workbooks SET snapshot = 'x', engine_version = 'old' WHERE id = $1::uuid RETURNING ''", &id)
        .await;
    assert!(verify_all(pool, &dir).await.is_err(), "a workbook with no backup must fail the verify");
    let version = text(pool, "SELECT engine_version FROM workbooks WHERE id = $1::uuid", &id).await;
    assert_eq!(version, "old", "a workbook the verify lost must keep its old engine version");
    std::fs::remove_dir_all(&dir).expect("clean");
}

async fn text(pool: &sqlx::PgPool, sql: &'static str, id: &str) -> String {
    sqlx::query_scalar(sql).bind(id).fetch_one(pool).await.expect(sql)
}
