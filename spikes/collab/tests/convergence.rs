use std::sync::Mutex;

use collab::{Axis, Edit, FULL_COLUMN_HEIGHT, Rect, Stats, Step, run};
use proptest::{
    prelude::*,
    test_runner::{Config, TestRunner},
};

fn cell_value() -> impl Strategy<Value = String> {
    prop_oneof![
        (-1000i32..1000).prop_map(|n| n.to_string()),
        "[a-z]{1,6}",
        Just(String::new()),
        Just("=A1+B2".to_string()),
        Just("=SUM(A1:C4)".to_string()),
        Just("=Sheet2!A1*2".to_string()),
        Just("=Alpha!B1".to_string()),
        Just("=A1&\"x\"".to_string()),
        Just("=1+".to_string()),
        Just("=IF(A1>3,B1,C1)".to_string()),
        Just("TRUE".to_string()),
        Just("12%".to_string()),
    ]
}

fn style() -> impl Strategy<Value = (String, String)> {
    prop_oneof![
        Just(("font.b", "true")),
        Just(("font.i", "true")),
        Just(("font.color", "#FF0000")),
        Just(("fill.fg_color", "#00FF00")),
        Just(("num_fmt", "0.00%")),
        Just(("alignment.horizontal", "center")),
        Just(("font.size", "14")),
    ]
    .prop_map(|(p, v)| (p.to_string(), v.to_string()))
}

fn structure() -> impl Strategy<Value = Edit> {
    let axis = prop_oneof![
        Just(Axis::InsertRows),
        Just(Axis::DeleteRows),
        Just(Axis::InsertColumns),
        Just(Axis::DeleteColumns),
    ];
    (0u32..4, axis, 1i32..9, 1i32..4).prop_map(|(sheet, axis, at, count)| Edit::Structure {
        sheet,
        axis,
        at,
        count,
    })
}

fn styled() -> impl Strategy<Value = Edit> {
    (
        0u32..4,
        1i32..9,
        1i32..9,
        1i32..4,
        1i32..4,
        style(),
        any::<bool>(),
    )
        .prop_map(|(sheet, row, column, width, height, (path, value), full)| {
            let (row, height) = if full {
                (1, FULL_COLUMN_HEIGHT)
            } else {
                (row, height)
            };
            Edit::Style {
                rect: Rect {
                    sheet,
                    row,
                    column,
                    width,
                    height,
                },
                path,
                value,
            }
        })
}

fn edit() -> impl Strategy<Value = Edit> {
    let rename = prop_oneof![Just("Alpha"), Just("Beta"), Just("Sheet2"), Just("Sheet1")];
    prop_oneof![
        6 => (0u32..4, 1i32..9, 1i32..9, cell_value())
            .prop_map(|(sheet, row, column, value)| Edit::Input { sheet, row, column, value }),
        2 => styled(),
        4 => structure(),
        1 => Just(Edit::NewSheet),
        1 => (0u32..4, rename).prop_map(|(sheet, name)| Edit::RenameSheet { sheet, name: name.to_string() }),
    ]
}

fn step() -> impl Strategy<Value = Step> {
    let client = 0usize..3;
    prop_oneof![
        2 => (client.clone(), edit()).prop_map(|(c, e)| Step::Edit(c, e)),
        1 => client.clone().prop_map(Step::Deliver),
        1 => client.prop_map(Step::Receive),
    ]
}

fn cases() -> u32 {
    std::env::var("COLLAB_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10_000)
}

#[test]
fn three_clients_converge() {
    let config = Config {
        cases: cases(),
        failure_persistence: None,
        ..Config::default()
    };
    let totals = Mutex::new(Stats::default());
    let result = TestRunner::new(config).run(&prop::collection::vec(step(), 1..60), |steps| {
        let stats = run(3, &steps).map_err(TestCaseError::fail)?;
        totals.lock().expect("lock").add(&stats);
        Ok(())
    });
    let totals = totals.into_inner().expect("lock");
    eprintln!("{} cases, totals: {totals:?}", cases());
    if let Err(e) = result {
        panic!("{e}");
    }
    assert!(totals.rejected > 0, "the reject path ran");
    assert!(totals.rebuilds > 0, "the rebuild path ran");
}
