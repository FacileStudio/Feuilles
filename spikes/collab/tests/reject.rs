use collab::{from_bytes, new_model};
use ironcalc::base::expressions::types::Area;

#[test]
fn a_failing_batch_can_change_the_model_before_it_errors() {
    let base = new_model().to_bytes();
    let mut author = from_bytes(&base);
    author.new_sheet().expect("sheet");
    author.flush_send_queue();
    let area = Area {
        sheet: 1,
        row: 1,
        column: 1,
        width: 1,
        height: 1,
    };
    author
        .update_range_style(&area, "font.b", "true")
        .expect("style");
    let style_on_missing_sheet = author.flush_send_queue();

    let mut server = from_bytes(&base);
    let error = server
        .apply_external_diffs(&style_on_missing_sheet)
        .expect_err("sheet 1 does not exist on the server");
    assert_eq!(error, "Invalid sheet index");
    assert_ne!(
        server.to_bytes(),
        base,
        "IronCalc 0.9.1 adds the style to the workbook before it finds the sheet is missing"
    );
}
