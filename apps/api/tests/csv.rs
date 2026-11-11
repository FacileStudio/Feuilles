use api::engine::{Model, export_csv, import_csv};
use ironcalc::base::types::CellType;

fn import(bytes: &[u8]) -> Model {
    import_csv(bytes, "Clients 2026", "en", "UTC", "en").expect("import")
}

fn cell(model: &Model, row: i32, column: i32) -> (String, CellType) {
    let value = model.get_formatted_cell_value(0, row, column).expect("value");
    (value, model.get_cell_type(0, row, column).expect("type"))
}

#[test]
fn a_french_excel_export_reads_with_its_types() {
    let file = "\u{feff}Nom;Montant;Tél;Code;Note\r\n\"Dupont; fils\";1,5;+33612345678;007;\"deux\r\nlignes\"\r\nX;=1+1;-3;TRUE;\r\n";
    let model = import(file.as_bytes());
    assert_eq!(model.get_model().workbook.worksheets[0].name, "Clients 2026");
    assert_eq!(cell(&model, 2, 1), ("Dupont; fils".into(), CellType::Text));
    assert_eq!(cell(&model, 2, 2), ("1.5".into(), CellType::Number));
    assert_eq!(cell(&model, 2, 3), ("+33612345678".into(), CellType::Text));
    assert_eq!(cell(&model, 2, 4), ("007".into(), CellType::Text));
    assert_eq!(cell(&model, 2, 5), ("deux\r\nlignes".into(), CellType::Text));
    assert_eq!(cell(&model, 3, 2), ("=1+1".into(), CellType::Text));
    assert_eq!(cell(&model, 3, 3), ("-3".into(), CellType::Number));
    assert_eq!(cell(&model, 3, 4), ("TRUE".into(), CellType::LogicalValue));
}

#[test]
fn a_formula_hidden_behind_a_sign_stays_text() {
    let model = import(b"+SUM(1;2),-A1,@cmd,-1.5e3\n");
    assert_eq!(model.get_cell_content(0, 1, 1).expect("content"), "'+SUM(1;2)");
    assert_eq!(model.get_cell_content(0, 1, 2).expect("content"), "'-A1");
    assert_eq!(cell(&model, 1, 3), ("@cmd".into(), CellType::Text));
    assert_eq!(cell(&model, 1, 4), ("-1.50E+03".into(), CellType::Number));
}

#[test]
fn an_export_reimports_to_the_same_csv() {
    let first = "name,amount,id,formula,quote\r\nAda,-12.5,007,'=HYPERLINK(\"x\"),''a\r\n\"x,y\",3,'+cmd,TRUE,\r\n";
    let once = export_csv(&import(first.as_bytes()), 0).expect("export");
    let twice = export_csv(&import(once.as_bytes()), 0).expect("export");
    assert_eq!(once, twice);
    assert!(once.contains("'=HYPERLINK"), "{once}");
    assert!(once.contains(",''a\r\n"), "{once}");
}

#[test]
fn too_many_columns_and_bad_bytes_are_refused_or_decoded() {
    let wide = ",".repeat(16_384);
    assert_eq!(import_csv(wide.as_bytes(), "W", "en", "UTC", "en").unwrap_err().code, tronc::Code::ResourceExhausted);
    assert_eq!(cell(&import(b"caf\xe9"), 1, 1).0, "café");
    assert_eq!(export_csv(&import(b""), 0).expect("export"), "");
}
