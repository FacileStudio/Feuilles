mod common;

use common::agent::{args, mounted, token};
use common::sheet::init;
use common::ws::{Event, connect};
use common::{person, space, state};
use serde_json::{Value, json};
use tronc::mcp::rmcp::model::CallToolResult;
use tronc::mcptest;

#[tokio::test(flavor = "multi_thread")]
async fn every_tool_declares_honest_hints() {
    let Some(state) = state().await else { return };
    let me = person(&state).await;
    let client = token(mounted(&state).await, me.user_id, "feuilles.read,feuilles.write").await;
    let tools = client.list_tools(None).await.expect("tools").tools;
    let hints = |name: &str| {
        let tool = tools.iter().find(|t| t.name == name).unwrap_or_else(|| panic!("{name} is not listed"));
        let a = tool.annotations.clone().expect("annotations");
        (a.read_only_hint, a.destructive_hint, a.idempotent_hint, a.open_world_hint)
    };
    let (read, add, arrange) = ((true, false, true), (false, false, false), (false, false, true));
    let (overwrite, remove) = ((false, true, true), (false, true, false));
    let expected = [
        (["feuilles_list_workbooks", "feuilles_get_workbook", "feuilles_read_range"].as_slice(), read),
        (&["feuilles_create_workbook", "feuilles_add_sheet", "feuilles_insert_rows", "feuilles_insert_columns"], add),
        (&["feuilles_update_sheet"], arrange),
        (
            &["feuilles_write_cells", "feuilles_clear_range", "feuilles_update_workbook", "feuilles_delete_workbook"],
            overwrite,
        ),
        (&["feuilles_delete_sheet", "feuilles_delete_rows", "feuilles_delete_columns"], remove),
    ];
    for (names, (r, d, i)) in expected {
        for name in names {
            assert_eq!(hints(name), (Some(r), Some(d), Some(i), Some(false)), "{name}");
        }
    }
    assert_eq!(tools.len(), 15);
}

#[tokio::test(flavor = "multi_thread")]
async fn workbooks_are_renamed_moved_and_trashed() {
    let Some(state) = state().await else { return };
    let (me, other) = (person(&state).await, person(&state).await);
    let team = space(&state, &[(&me, "owner"), (&other, "member")]).await;
    let client = token(mounted(&state).await, me.user_id, "feuilles.read,feuilles.write").await;
    let created = mcptest::call(&client, "feuilles_create_workbook", args(json!({"title": "Plan"}))).await;
    let id = mcptest::decode::<Value>(&created)["id"].as_str().expect("id").to_owned();
    let update = json!({"workbook_id": id, "title": "Budget", "space_id": team});
    let moved = mcptest::decode::<Value>(&mcptest::call(&client, "feuilles_update_workbook", args(update)).await);
    assert_eq!((moved["title"].as_str(), moved["space_id"].as_str()), (Some("Budget"), Some(team.as_str())));
    let stranger = json!({"workbook_id": id, "space_id": "nowhere"});
    let refused = mcptest::call(&client, "feuilles_update_workbook", args(stranger)).await;
    assert!(mcptest::text(&refused).contains("Space 'nowhere' not found"), "{}", mcptest::text(&refused));
    let home = json!({"workbook_id": id, "space_id": ""});
    let back = mcptest::decode::<Value>(&mcptest::call(&client, "feuilles_update_workbook", args(home)).await);
    assert_eq!(back["space_id"], Value::Null);
    for _ in 0..2 {
        let trashed = mcptest::call(&client, "feuilles_delete_workbook", args(json!({"workbook_id": id}))).await;
        assert!(mcptest::decode::<Value>(&trashed)["trashed_at"].is_string());
    }
    let write = json!({"workbook_id": id, "writes": [{"sheet": 0, "row": 1, "column": 1, "input": "x"}]});
    let refused = mcptest::call(&client, "feuilles_write_cells", args(write)).await;
    assert!(mcptest::text(&refused).contains("in the trash"), "{}", mcptest::text(&refused));
    let unknown = json!({"workbook_id": uuid::Uuid::new_v4()});
    let missing = mcptest::call(&client, "feuilles_delete_workbook", args(unknown)).await;
    assert!(mcptest::text(&missing).contains("feuilles_list_workbooks"), "{}", mcptest::text(&missing));
}

#[tokio::test(flavor = "multi_thread")]
async fn sheets_are_renamed_reordered_and_deleted_live() {
    let Some(state) = state().await else { return };
    let me = person(&state).await;
    let addr = mounted(&state).await;
    let client = token(addr, me.user_id, "feuilles.read,feuilles.write").await;
    let created = mcptest::call(&client, "feuilles_create_workbook", args(json!({"title": "Tabs"}))).await;
    let id = mcptest::decode::<Value>(&created)["id"].as_str().expect("id").to_owned();
    mcptest::call(&client, "feuilles_add_sheet", args(json!({"workbook_id": id, "name": "Notes"}))).await;
    let mut browser = connect(addr, &id, &me.token).await;
    init(&mut browser).await;
    let update = json!({"workbook_id": id, "sheet": "notes", "position": 0, "name": "Ideas"});
    let updated = mcptest::decode::<Value>(&mcptest::call(&client, "feuilles_update_sheet", args(update)).await);
    assert_eq!(updated["sheets"][0]["name"], "Ideas");
    assert!(matches!(browser.next_data().await, Event::Update { client_id: 0, .. }), "the browser missed it");
    let missing =
        mcptest::call(&client, "feuilles_delete_sheet", args(json!({"workbook_id": id, "sheet": "Foo"}))).await;
    assert_eq!(mcptest::text(&missing), "Sheet 'Foo' not found; sheets are: 'Ideas', 'Sheet1'.");
    let deleted = mcptest::call(&client, "feuilles_delete_sheet", args(json!({"workbook_id": id, "sheet": 1}))).await;
    assert_eq!(mcptest::decode::<Value>(&deleted)["sheets"].as_array().map(Vec::len), Some(1));
    let last = mcptest::call(&client, "feuilles_delete_sheet", args(json!({"workbook_id": id, "sheet": 0}))).await;
    assert!(mcptest::text(&last).contains("at least one sheet"), "{}", mcptest::text(&last));
}

#[tokio::test(flavor = "multi_thread")]
async fn rows_and_columns_are_inserted_and_deleted_and_ranges_cleared() {
    let Some(state) = state().await else { return };
    let me = person(&state).await;
    let client = token(mounted(&state).await, me.user_id, "feuilles.read,feuilles.write").await;
    let created = mcptest::call(&client, "feuilles_create_workbook", args(json!({"title": "Grid"}))).await;
    let id = mcptest::decode::<Value>(&created)["id"].as_str().expect("id").to_owned();
    let writes =
        json!([{"sheet": 0, "row": 1, "column": 1, "input": "a"}, {"sheet": 0, "row": 2, "column": 2, "input": "b"}]);
    mcptest::call(&client, "feuilles_write_cells", args(json!({"workbook_id": id, "writes": writes}))).await;
    let edit = |at: i32, count: i32| args(json!({"workbook_id": id, "sheet": 0, "at": at, "count": count}));
    let used = |result: &CallToolResult| mcptest::decode::<Value>(result)["sheets"][0]["used_range"].clone();
    assert_eq!(used(&mcptest::call(&client, "feuilles_insert_rows", edit(1, 2)).await), "A3:B4");
    assert_eq!(used(&mcptest::call(&client, "feuilles_insert_columns", edit(1, 1)).await), "B3:C4");
    assert_eq!(used(&mcptest::call(&client, "feuilles_delete_rows", edit(3, 1)).await), "C3:C3");
    assert_eq!(used(&mcptest::call(&client, "feuilles_delete_columns", edit(1, 2)).await), "A3:A3");
    let refused = mcptest::call(&client, "feuilles_insert_rows", edit(1, 20_000)).await;
    assert_eq!(mcptest::text(&refused), "Count must be 1 to 10000");
    let clear = json!({"workbook_id": id, "sheet": "Sheet1", "range": "A1:XFD1048576"});
    for _ in 0..2 {
        mcptest::decode::<Value>(&mcptest::call(&client, "feuilles_clear_range", args(clear.clone())).await);
    }
    let read = mcptest::call(&client, "feuilles_read_range", args(json!({"workbook_id": id, "range": "A3"}))).await;
    assert_eq!(mcptest::decode::<Value>(&read)["rows"][0][0]["input"], "");
}

#[tokio::test(flavor = "multi_thread")]
async fn reads_clamp_to_the_data_and_lists_page() {
    let Some(state) = state().await else { return };
    let me = person(&state).await;
    let client = token(mounted(&state).await, me.user_id, "feuilles.read,feuilles.write").await;
    let mut id = String::new();
    for title in ["One", "Two", "Three"] {
        let created = mcptest::call(&client, "feuilles_create_workbook", args(json!({"title": title}))).await;
        id = mcptest::decode::<Value>(&created)["id"].as_str().expect("id").to_owned();
    }
    let writes: Vec<Value> = (1..=3).map(|row| json!({"sheet": 0, "row": row, "column": 2, "input": "x"})).collect();
    mcptest::call(&client, "feuilles_write_cells", args(json!({"workbook_id": id, "writes": writes}))).await;
    let read = json!({"workbook_id": id, "sheet": "sheet1", "range": "A1:XFD1048576"});
    let page = mcptest::decode::<Value>(&mcptest::call(&client, "feuilles_read_range", args(read)).await);
    assert_eq!((page["range"].as_str(), page["next_cursor"].as_str()), (Some("A1:B3"), None));
    let (mut seen, mut cursor) = (0, None::<String>);
    loop {
        let input = args(json!({"limit": 2, "cursor": cursor}));
        let listed = mcptest::decode::<Value>(&mcptest::call(&client, "feuilles_list_workbooks", input).await);
        seen += listed["workbooks"].as_array().expect("workbooks").len();
        cursor = listed["next_cursor"].as_str().map(str::to_owned);
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(seen, 3);
}
