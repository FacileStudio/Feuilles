use collab::{from_bytes, new_model};

#[test]
fn volatile_formulas_converge_on_inputs_but_not_on_values() {
    let base = new_model().to_bytes();
    let mut values_differ = false;
    for _ in 0..5 {
        let mut author = from_bytes(&base);
        author
            .set_user_input(0, 1, 1, "=RANDBETWEEN(1,1000000000)")
            .expect("input");
        let batch = author.flush_send_queue();
        let mut replica = from_bytes(&base);
        replica.apply_external_diffs(&batch).expect("apply");
        assert_eq!(
            author.get_cell_content(0, 1, 1),
            replica.get_cell_content(0, 1, 1)
        );
        values_differ |= author.to_bytes() != replica.to_bytes();
    }
    assert!(values_differ, "each replica rolls its own random value");
}
