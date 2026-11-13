mod common;

use common::agent::{args, mounted, token};
use common::sheet::init;
use common::web::{call, create};
use common::ws::{Event, connect};
use common::{app, person, state};
use http::Method;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use tronc::mcptest;

async fn stored(pool: &PgPool, id: &str) -> Option<i32> {
    sqlx::query_scalar("SELECT sheet_count FROM workbooks WHERE id = $1::uuid")
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("sheet_count")
}

#[tokio::test(flavor = "multi_thread")]
async fn the_migration_adds_a_nullable_sheet_count() {
    let Some(state) = state().await else { return };
    let column: (String, String) = sqlx::query_as(
        "SELECT data_type::text, is_nullable::text FROM information_schema.columns
         WHERE table_schema = current_schema() AND table_name = 'workbooks' AND column_name = 'sheet_count'",
    )
    .fetch_one(&state.pool)
    .await
    .expect("the column");
    assert_eq!(column, ("integer".to_owned(), "YES".to_owned()));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_count_follows_rest_mcp_and_the_room() {
    let Some(app) = app().await else { return };
    let (pool, me) = (&app.state.pool, person(&app.state).await);
    let id = create(&app.router, &me, None).await["id"].as_str().expect("id").to_owned();
    let (_, listed) = call(&app.router, Method::GET, "/api/workbooks", Some(&me.token), None).await;
    let row = listed["workbooks"].as_array().expect("list").iter().find(|w| w["id"] == id.as_str()).cloned();
    assert_eq!(row.map(|w| (w["sheet_count"].clone(), w["sheets"].clone())), Some((json!(1), json!([]))));
    let addr = mounted(&app.state).await;
    let client = token(addr, me.user_id, "feuilles.read,feuilles.write").await;
    mcptest::call(&client, "feuilles_add_sheet", args(json!({"workbook_id": id, "name": "Notes"}))).await;
    assert_eq!(stored(pool, &id).await, Some(2));
    mcptest::call(&client, "feuilles_delete_sheet", args(json!({"workbook_id": id, "sheet": "Notes"}))).await;
    assert_eq!(stored(pool, &id).await, Some(1));
    let mut browser = connect(addr, &id, &me.token).await;
    let (_, _, mut model) = init(&mut browser).await;
    api::engine::new_sheet(&mut model, Some("Live")).expect("sheet");
    browser.push(1, &model.flush_send_queue()).await;
    assert!(matches!(browser.next_data().await, Event::Update { client_seq: 1, .. }), "the push was refused");
    assert_eq!(stored(pool, &id).await, Some(2));
    let created = mcptest::call(&client, "feuilles_create_workbook", args(json!({"title": "Agent"}))).await;
    let agent = mcptest::decode::<Value>(&created)["id"].as_str().expect("id").to_owned();
    assert_eq!(stored(pool, &agent).await, Some(1));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_backfill_counts_an_old_row_from_its_snapshot_and_log() {
    let Some(app) = app().await else { return };
    let (pool, me) = (&app.state.pool, person(&app.state).await);
    let id = create(&app.router, &me, None).await["id"].as_str().expect("id").to_owned();
    let client = token(mounted(&app.state).await, me.user_id, "feuilles.read,feuilles.write").await;
    for name in ["Two", "Three"] {
        mcptest::call(&client, "feuilles_add_sheet", args(json!({"workbook_id": id, "name": name}))).await;
    }
    sqlx::query("UPDATE workbooks SET sheet_count = NULL WHERE id = $1::uuid")
        .bind(&id)
        .execute(pool)
        .await
        .expect("age");
    let filled = api::backfill::fill(pool, &CancellationToken::new(), std::time::Duration::ZERO).await.expect("fill");
    assert!(filled >= 1, "filled {filled}");
    assert_eq!(stored(pool, &id).await, Some(3));
    let stopped = CancellationToken::new();
    stopped.cancel();
    sqlx::query("UPDATE workbooks SET sheet_count = NULL WHERE id = $1::uuid")
        .bind(&id)
        .execute(pool)
        .await
        .expect("age");
    assert_eq!(api::backfill::fill(pool, &stopped, std::time::Duration::ZERO).await.expect("fill"), 0);
    assert_eq!(stored(pool, &id).await, None);
}
