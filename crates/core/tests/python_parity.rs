//! Đối chiếu với bản Python cũ: sai khác ≤ 0,01 m³/s (đặc tả 10.2).
//! Ca sinh bằng `scripts/gen_python_cases.py`, không phải ca thật của người vận hành.

use luuluong_core::{compute, Inputs, Turbine, VolumeTable};

#[test]
fn matches_old_python_on_table_points() {
    let table = VolumeTable::builtin();
    let mut reader = csv::Reader::from_path("tests/data/python_cases.csv").unwrap();
    let mut count = 0;
    for row in reader.records() {
        let row = row.unwrap();
        let n = |i: usize| row[i].parse::<f64>().unwrap();
        let inputs = Inputs {
            z_start: n(0),
            z_end: n(1),
            hours: n(2),
            h1: n(3),
            h2: n(4),
            spill_q: 0.0,
            spill_h: 0.0,
            eco_q: 0.0,
        };
        let q = compute(&table, &Turbine::default(), &inputs).unwrap().q_in;
        assert!((q - n(5)).abs() <= 0.01, "{inputs_row:?}: Rust {q} vs Python {}", n(5), inputs_row = &row);
        count += 1;
    }
    assert_eq!(count, 50);
}
