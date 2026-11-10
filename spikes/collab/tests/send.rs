use collab::{LANGUAGE, new_model};
use ironcalc::base::UserModel;

const fn assert_send_sync<T: Send + Sync>() {}
const _: () = assert_send_sync::<UserModel<'static>>();

#[test]
fn user_model_moves_to_another_thread() {
    let mut model = new_model();
    model.set_user_input(0, 1, 1, "=6*7").expect("input");
    let bytes = std::thread::spawn(move || model.to_bytes())
        .join()
        .expect("thread");
    let back = UserModel::from_bytes(&bytes, LANGUAGE).expect("load");
    assert_eq!(back.get_formatted_cell_value(0, 1, 1).expect("value"), "42");
}
