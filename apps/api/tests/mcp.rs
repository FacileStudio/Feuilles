mod common;

use std::sync::Arc;

use api::engine::CellWrite;
use api::hub::Change;
use api::modules::workbooks::types::CreateWorkbook;
use api::modules::workbooks::{actions, model};
use common::agent::{args, mounted, token};
use common::sheet::init;
use common::ws::{Event, connect};
use common::{person, state};
use serde_json::{Value, json};
use tronc::mcp::rmcp::model::CallToolRequestParams;
use tronc::mcptest;

async fn seq(pool: &sqlx::PgPool, id: &str) -> i64 {
    sqlx::query_scalar("SELECT seq FROM workbooks WHERE id = $1::uuid").bind(id).fetch_one(pool).await.expect("seq")
}

async fn target(state: api::state::AppState, user: i64, extra: Value) -> mcptest::Target {
    let me = porte::Identity { user_id: user, ..porte::Identity::default() };
    let body = CreateWorkbook { title: "Target".into(), space_id: None, locale: None, timezone: None, language: None };
    let id = actions::create(&state, &me, &body).await.expect("workbook").id;
    let cells = (1..=3).map(|row| Change::Cell(CellWrite { sheet: 0, row, column: 1, input: "x".into() }));
    let changes = cells.chain([Change::AddSheet(None)]).collect();
    model::change(&state, id, user, changes).await.expect("cells and a sheet");
    let id = id.to_string();
    let mut input = json!({"workbook_id": id, "sheet": 0});
    input.as_object_mut().expect("object").extend(extra.as_object().expect("object").clone());
    let before = seq(&state.pool, &id).await;
    let runs = move || -> mcptest::Fut<usize> {
        let (pool, id) = (state.pool.clone(), id.clone());
        Box::pin(async move { (seq(&pool, &id).await - before) as usize })
    };
    mcptest::Target { args: args(input), runs: Arc::new(runs) }
}

fn case(state: &api::state::AppState, user: i64, tool: &str, extra: Value) -> mcptest::ConfirmCase {
    let state = state.clone();
    let setup = move || -> mcptest::Fut<mcptest::Target> { Box::pin(target(state.clone(), user, extra.clone())) };
    mcptest::ConfirmCase { tool: tool.into(), setup: Arc::new(setup) }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_endpoint_passes_the_tronc_conformance_kit_with_its_deletes_confirmed() {
    let Some(state) = state().await else { return };
    let addr = mounted(&state).await;
    let cases = [
        ("feuilles_delete_sheet", json!({})),
        ("feuilles_delete_rows", json!({"at": 1, "count": 2})),
        ("feuilles_delete_columns", json!({"at": 1})),
        ("feuilles_clear_range", json!({"range": "A1:A3"})),
    ];
    for (tool, extra) in cases {
        let user = person(&state).await.user_id;
        let token: mcptest::TokenFn = Arc::new(move |scopes| match scopes.is_empty() {
            true => format!("{user}|feuilles.read,feuilles.write"),
            false => format!("{user}|feuilles.read,{}", scopes.join(",")),
        });
        let confirm = vec![case(&state, user, tool, extra)];
        mcptest::run(&mcptest::Config { url: format!("http://{addr}/mcp"), token, confirm }).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_read_token_reads_and_is_refused_a_write_and_a_write_token_writes_live() {
    let Some(state) = state().await else { return };
    let me = person(&state).await;
    let addr = mounted(&state).await;
    let writer = token(addr, me.user_id, "feuilles.read,feuilles.write").await;
    let created = mcptest::call(&writer, "feuilles_create_workbook", args(json!({"title": "Agent book"}))).await;
    let id = mcptest::decode::<Value>(&created)["id"].as_str().expect("id").to_owned();
    let mut browser = connect(addr, &id, &me.token).await;
    init(&mut browser).await;
    let reader = token(addr, me.user_id, "feuilles.read").await;
    let write = json!({"workbook_id": id, "writes": [{"sheet": 0, "row": 1, "column": 1, "input": "=6*7"}]});
    let refused =
        reader.call_tool(CallToolRequestParams::new("feuilles_write_cells").with_arguments(args(write.clone()))).await;
    assert!(
        refused.is_err() || refused.as_ref().is_ok_and(|r| r.is_error == Some(true)),
        "a read token wrote: {refused:?}"
    );
    let written = mcptest::call(&writer, "feuilles_write_cells", args(write)).await;
    assert_eq!(mcptest::decode::<Value>(&written)["seq"], 1);
    assert!(
        matches!(browser.next_data().await, Event::Update { seq: 1, client_id: 0, .. }),
        "the browser did not see the write"
    );
    read_back(&reader, &id).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn ranges_page_under_the_budget_and_sheets_are_added() {
    let Some(state) = state().await else { return };
    let me = person(&state).await;
    let addr = mounted(&state).await;
    let client = token(addr, me.user_id, "feuilles.read,feuilles.write").await;
    let created = mcptest::call(&client, "feuilles_create_workbook", args(json!({"title": "Big"}))).await;
    let id = mcptest::decode::<Value>(&created)["id"].as_str().expect("id").to_owned();
    let writes: Vec<Value> =
        (1..=2000).map(|row| json!({"sheet": 0, "row": row, "column": 1, "input": "x".repeat(60)})).collect();
    mcptest::call(&client, "feuilles_write_cells", args(json!({"workbook_id": id, "writes": writes}))).await;
    let (rows, pages) = read_all_pages(&client, &id).await;
    assert_eq!(rows, 2000);
    assert!(pages > 1);
    let added = mcptest::call(&client, "feuilles_add_sheet", args(json!({"workbook_id": id, "name": "Notes"}))).await;
    assert_eq!(mcptest::decode::<Value>(&added)["sheets"][1]["name"], "Notes");
    let listed = mcptest::call(&client, "feuilles_list_workbooks", args(json!({}))).await;
    assert_eq!(mcptest::decode::<Value>(&listed)["workbooks"][0]["id"], id.as_str());
}

async fn read_all_pages(client: &mcptest::Client, id: &str) -> (usize, usize) {
    let (mut rows, mut cursor, mut pages) = (0, None::<String>, 0);
    loop {
        let input = args(json!({"workbook_id": id, "range": "A1:A2000", "cursor": cursor}));
        let page = mcptest::call(client, "feuilles_read_range", input).await;
        assert!(mcptest::text(&page).len() < 2 * tronc::mcp::RESULT_BUDGET);
        let page = mcptest::decode::<Value>(&page);
        rows += page["rows"].as_array().expect("rows").len();
        pages += 1;
        cursor = page["next_cursor"].as_str().map(str::to_owned);
        assert_eq!(page["hint"].is_string(), cursor.is_some());
        if cursor.is_none() {
            return (rows, pages);
        }
    }
}

async fn read_back(reader: &mcptest::Client, id: &str) {
    let read = mcptest::call(reader, "feuilles_read_range", args(json!({"workbook_id": id, "range": "A1"}))).await;
    let cell = json!({"input": "=6*7", "value": 42.0, "formatted": "42"});
    assert_eq!(mcptest::decode::<Value>(&read)["rows"][0][0], cell);
    let got = mcptest::call(reader, "feuilles_get_workbook", args(json!({"workbook_id": id}))).await;
    assert_eq!(mcptest::decode::<Value>(&got)["sheets"][0]["used_range"], "A1:A1");
}
