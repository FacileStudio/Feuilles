#![deny(unused, dead_code)]

use std::{fs, path::PathBuf, time::Instant};

use collab::LANGUAGE;
use ironcalc::base::{Model, UserModel};

const COLUMNS: i32 = 100;

fn cell_input(row: i32, column: i32) -> String {
    match column % 3 {
        0 => format!("{}", row * column),
        1 => format!("t{row}"),
        _ => format!("=A{row}+1"),
    }
}

fn filled(cells: usize) -> UserModel<'static> {
    let mut model = Model::new_empty("Bench", "en", "UTC", LANGUAGE).expect("model");
    for i in 0..cells as i32 {
        let (row, column) = (i / COLUMNS + 1, i % COLUMNS + 1);
        model
            .set_user_input(0, row, column, cell_input(row, column))
            .expect("input");
    }
    model.evaluate();
    UserModel::from_model(model)
}

fn median_ms(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[samples.len() / 2]
}

fn measure(model: &UserModel<'static>, runs: usize) -> (Vec<u8>, f64, f64) {
    let mut bytes = Vec::new();
    let to = (0..runs)
        .map(|_| {
            let t = Instant::now();
            bytes = model.to_bytes();
            t.elapsed().as_secs_f64() * 1000.0
        })
        .collect();
    let from = (0..runs)
        .map(|_| {
            let t = Instant::now();
            let loaded = UserModel::from_bytes(&bytes, LANGUAGE).expect("load");
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            drop(loaded);
            ms
        })
        .collect();
    (bytes, median_ms(to), median_ms(from))
}

fn main() {
    let out: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/bench"));
    fs::create_dir_all(&out).expect("dir");
    println!("cells\tbytes\tto_bytes_ms\tfrom_bytes_ms");
    for (cells, runs) in [(1_000usize, 51), (100_000, 11), (1_000_000, 5)] {
        let (bytes, to, from) = measure(&filled(cells), runs);
        fs::write(out.join(format!("snap-{cells}.bin")), &bytes).expect("write");
        println!("{cells}\t{}\t{to:.3}\t{from:.3}", bytes.len());
    }
}
