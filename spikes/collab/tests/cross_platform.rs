use std::{fs, path::Path, process::Command};

use collab::{from_bytes, new_model};
use ironcalc::base::{UserModel, expressions::types::Area};

fn cell_line(model: &UserModel<'static>, sheet: u32, row: i32, column: i32) -> Option<String> {
    let content = model.get_cell_content(sheet, row, column).expect("cell");
    if content.is_empty() {
        return None;
    }
    let formatted = model
        .get_formatted_cell_value(sheet, row, column)
        .expect("value");
    Some(format!("{sheet}\t{row}\t{column}\t{content}\t{formatted}"))
}

fn dump_cells(model: &UserModel<'static>) -> String {
    let mut lines = Vec::new();
    for (index, sheet) in model.get_worksheets_properties().iter().enumerate() {
        let index = index as u32;
        lines.push(format!("sheet\t{index}\t{}", sheet.name));
        let cells = (1..=12).flat_map(|row| (1..=12).map(move |column| (row, column)));
        lines.extend(cells.filter_map(|(row, column)| cell_line(model, index, row, column)));
    }
    lines.join("\n")
}

fn area(sheet: u32, row: i32, column: i32, width: i32, height: i32) -> Area {
    Area {
        sheet,
        row,
        column,
        width,
        height,
    }
}

fn native_edits(model: &mut UserModel<'static>) -> Result<(), String> {
    model.set_user_input(0, 1, 1, "12")?;
    model.set_user_input(0, 1, 2, "=A1*3")?;
    model.set_user_input(0, 2, 1, "héllo wörld ✓")?;
    model.set_user_input(0, 3, 3, "=CONCATENATE(A2,\"-\",B1)")?;
    model.set_user_input(0, 4, 1, "0.1")?;
    model.set_user_input(0, 5, 1, "=A4+0.2")?;
    model.update_range_style(&area(0, 1, 1, 3, 2), "fill.fg_color", "#FFCC00")?;
    model.update_range_style(&area(0, 5, 1, 1, 1), "num_fmt", "0.000")?;
    model.insert_columns(0, 2, 1)?;
    model.delete_rows(0, 4, 1)?;
    model.new_sheet()?;
    model.rename_sheet(1, "Résumé")?;
    model.set_user_input(1, 2, 2, "=SUM(Sheet1!A1:D5)")?;
    model.insert_rows(0, 1, 2)?;
    Ok(())
}

fn run_wasm_side(dir: &Path) {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("js/xplat.mjs");
    let runtime = std::env::var("COLLAB_JS").unwrap_or_else(|_| "bun".to_string());
    let output = Command::new(runtime)
        .arg(&script)
        .arg(dir)
        .output()
        .expect("the JS runtime starts; run `bun install` in js/ first");
    assert!(
        output.status.success(),
        "wasm side failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_native_side(dir: &Path, base: &[u8]) {
    fs::write(dir.join("base.bin"), base).expect("write");
    let mut native = from_bytes(base);
    native_edits(&mut native).expect("native edits");
    let native_diff = native.flush_send_queue();
    fs::write(dir.join("native_diff.bin"), &native_diff).expect("write");
    fs::write(dir.join("native_cells.txt"), dump_cells(&native)).expect("write");
    fs::write(dir.join("native_after.bin"), native.to_bytes()).expect("write");
}

fn check_wasm_side(dir: &Path, base: &[u8]) {
    let wasm_diff = fs::read(dir.join("wasm_diff.bin")).expect("read");
    let wasm_cells = fs::read_to_string(dir.join("wasm_cells.txt")).expect("read");
    let wasm_after = fs::read(dir.join("wasm_after.bin")).expect("read");

    let mut replay = from_bytes(base);
    replay
        .apply_external_diffs(&wasm_diff)
        .expect("wasm diff applies natively");
    assert_eq!(
        dump_cells(&replay),
        wasm_cells,
        "wasm diff gives the same cells natively"
    );
    assert_eq!(
        replay.to_bytes(),
        wasm_after,
        "wasm diff gives the same bytes natively"
    );

    let snapshot = from_bytes(&wasm_after);
    assert_eq!(
        dump_cells(&snapshot),
        wasm_cells,
        "wasm snapshot loads natively"
    );
    assert!(
        wasm_cells.contains("Données"),
        "wasm edits reached the native side"
    );
}

#[test]
fn diffs_cross_between_wasm_and_native() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("xplat");
    fs::create_dir_all(&dir).expect("dir");
    let base = new_model().to_bytes();
    write_native_side(&dir, &base);
    run_wasm_side(&dir);
    check_wasm_side(&dir, &base);
}
