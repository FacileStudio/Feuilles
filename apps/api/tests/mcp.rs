mod common;

use std::sync::Arc;

use common::agent::{args, mounted, token};
use common::sheet::init;
use common::ws::{Event, connect};
use common::{person, state};
use serde_json::{Value, json};
use tronc::mcp::rmcp::model::CallToolRequestParams;
use tronc::mcptest;

#[tokio::test(flavor = "multi_thread")]
async fn the_endpoint_passes_the_tronc_conformance_kit() {
    let Some(state) = state().await else { return };
    let me = person(&state).await;
    let addr = mounted(&state).await;
    let user = me.user_id.to_string();
    let token: mcptest::TokenFn = Arc::new(move |scopes| format!("{user}|feuilles.read,{}", scopes.join(",")));
    mcptest::run(&mcptest::Config { url: format!("http://{addr}/mcp"), token, confirm: Vec::new() }).await;
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
