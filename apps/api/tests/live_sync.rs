mod common;

use common::sheet::{batch, init};
use common::web::{call, create};
use common::ws::{Event, connect};
use common::{person, serve, state};
use http::{Method, StatusCode};
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn two_sockets_see_each_batch_in_one_order_and_api_writes_live() {
    let Some(state) = state().await else { return };
    let addr = serve(state.clone()).await;
    let router = api::router::build_router(state.clone(), Default::default()).expect("router");
    let me = person(&state).await;
    let id = create(&router, &me, None).await["id"].as_str().expect("id").to_owned();
    let (mut one, mut two) = (connect(addr, &id, &me.token).await, connect(addr, &id, &me.token).await);
    let (first, role, mut model_one) = init(&mut one).await;
    let (second, _, mut model_two) = init(&mut two).await;
    assert_eq!(role, 2);
    assert_ne!(first, second);
    let payload = batch(&mut model_one, 1, "41");
    one.push(7, &payload).await;
    for ws in [&mut one, &mut two] {
        assert_eq!(
            ws.next_data().await,
            Event::Update { seq: 1, client_id: first, client_seq: 7, payload: payload.clone() }
        );
    }
    model_two.apply_external_diffs(&payload).expect("replay");
    let writes = json!({"writes": [{"sheet": 0, "row": 2, "column": 1, "input": "=A1+1"}]});
    let path = format!("/api/workbooks/{id}/cells");
    assert_eq!(call(&router, Method::PUT, &path, Some(&me.token), Some(writes)).await.1, json!({"seq": 2}));
    for (ws, model) in [(&mut one, &mut model_one), (&mut two, &mut model_two)] {
        server_update(ws, model).await;
    }
    let (_, cells) = call(&router, Method::GET, &format!("{path}?range=A1:A2"), Some(&me.token), None).await;
    assert_eq!(cells["rows"][1][0]["formatted"], "42");
    assert_eq!(model_one.to_bytes(), model_two.to_bytes());
}

#[tokio::test(flavor = "multi_thread")]
async fn presence_lists_everyone_with_their_cursor() {
    let Some(state) = state().await else { return };
    let addr = serve(state.clone()).await;
    let router = api::router::build_router(state.clone(), Default::default()).expect("router");
    let (me, you) = (person(&state).await, person(&state).await);
    let shared = common::space(&state, &[(&me, "owner"), (&you, "member")]).await;
    let id = create(&router, &me, Some(&shared)).await["id"].as_str().expect("id").to_owned();
    let mut one = connect(addr, &id, &me.token).await;
    let mut two = connect(addr, &id, &you.token).await;
    let cursor = br#"{"sheet":0,"row":3,"column":2,"range":[3,2,4,4]}"#;
    two.send([&[0x02][..], cursor].concat()).await;
    let want = json!({"sheet": 0, "row": 3, "column": 2, "range": [3, 2, 4, 4]});
    loop {
        if let Event::Presence(list) = one.next().await {
            let names: Vec<&str> = list.as_array().expect("list").iter().filter_map(|p| p["name"].as_str()).collect();
            if names.len() == 2 && list.as_array().expect("list").iter().any(|p| p["cursor"] == want) {
                assert!(names.contains(&you.subject.as_str()));
                break;
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_last_to_leave_writes_the_snapshot_and_shutdown_closes_with_1001() {
    let Some(state) = state().await else { return };
    let addr = serve(state.clone()).await;
    let router = api::router::build_router(state.clone(), Default::default()).expect("router");
    let me = person(&state).await;
    let id = create(&router, &me, None).await["id"].as_str().expect("id").to_owned();
    let mut ws = connect(addr, &id, &me.token).await;
    let (_, _, mut model) = init(&mut ws).await;
    ws.push(1, &batch(&mut model, 1, "1")).await;
    ws.next_data().await;
    drop(ws);
    let uuid: uuid::Uuid = id.parse().expect("uuid");
    assert_eq!(snapshot_reaches(&state.pool, uuid, 1).await, 1);
    let mut ws = connect(addr, &id, &me.token).await;
    let (_, _, mut model) = init(&mut ws).await;
    assert_eq!(model.get_formatted_cell_value(0, 1, 1).expect("value"), "1");
    ws.push(2, &batch(&mut model, 2, "2")).await;
    ws.next_data().await;
    let hub = state.hub.clone();
    tokio::task::spawn_blocking(move || hub.shutdown()).await.expect("shutdown");
    assert_eq!(ws.closed().await, Some(1001));
    assert_eq!(snapshot_reaches(&state.pool, uuid, 2).await, 2);
    let (status, _) = call(&router, Method::GET, &format!("/api/workbooks/{id}"), Some(&me.token), None).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

async fn snapshot_reaches(pool: &sqlx::PgPool, id: uuid::Uuid, want: i64) -> i64 {
    let mut snapshot_seq = 0;
    for _ in 0..100 {
        snapshot_seq = sqlx::query_scalar("SELECT snapshot_seq FROM workbooks WHERE id = $1 AND seq = $2")
            .bind(id)
            .bind(want)
            .fetch_optional(pool)
            .await
            .expect("seq")
            .unwrap_or(0);
        if snapshot_seq == want {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    snapshot_seq
}

async fn server_update(ws: &mut common::ws::Ws, model: &mut api::engine::Model) {
    let Event::Update { seq: 2, client_id: 0, client_seq: 0, payload } = ws.next_data().await else {
        panic!("no server update")
    };
    model.apply_external_diffs(&payload).expect("replay");
    assert_eq!(model.get_formatted_cell_value(0, 2, 1).expect("value"), "42");
}
