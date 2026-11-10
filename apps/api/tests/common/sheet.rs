use super::ws::{Event, Ws};

pub fn model(snapshot: &[u8]) -> api::engine::Model {
    api::engine::load(snapshot, "en").expect("snapshot")
}

pub async fn init(ws: &mut Ws) -> (u32, u8, api::engine::Model) {
    match ws.next_data().await {
        Event::Init { client_id, role, snapshot, .. } => (client_id, role, model(&snapshot)),
        other => panic!("expected INIT, got {other:?}"),
    }
}

pub fn batch(model: &mut api::engine::Model, row: i32, input: &str) -> Vec<u8> {
    model.set_user_input(0, row, 1, input).expect("input");
    model.flush_send_queue()
}
