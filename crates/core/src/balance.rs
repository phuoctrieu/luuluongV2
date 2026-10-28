use crate::error::CalcError;
use crate::reservoir::VolumeTable;

/// Lưu lượng mỗi tổ máy (m³/s).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Turbine {
    pub q_h1: f64,
    pub q_h2: f64,
}

impl Default for Turbine {
    fn default() -> Self {
        Self { q_h1: 17.3, q_h2: 17.3 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Inputs {
    pub z_start: f64,
    pub z_end: f64, // m
    pub hours: f64, // giờ tính toán
    pub h1: f64,
    pub h2: f64, // giờ chạy máy
    pub spill_q: f64,
    pub spill_h: f64, // xả tràn: m3/s, giờ
    pub eco_q: f64,   // xả dòng chảy tối thiểu: m3/s
}

#[derive(Debug)]
pub struct Outputs {
    pub v_start: f64,
    pub v_end: f64, // m3
    pub delta_v: f64,
    pub v_turbine: f64,
    pub v_release: f64,
    pub q_in: f64, // m3/s
    pub w_in: f64, // m3
}

pub fn compute(t: &VolumeTable, cfg: &Turbine, i: &Inputs) -> Result<Outputs, CalcError> {
    let all = [i.z_start, i.z_end, i.hours, i.h1, i.h2, i.spill_q, i.spill_h, i.eco_q];
    if all.iter().any(|x| !x.is_finite()) {
        return Err(CalcError::InvalidNumber);
    }
    if i.hours <= 0.0 {
        return Err(CalcError::HoursNotPositive);
    }
    if [i.h1, i.h2, i.spill_q, i.spill_h, i.eco_q].iter().any(|x| *x < 0.0) {
        return Err(CalcError::Negative);
    }
    if i.h1 > i.hours || i.h2 > i.hours {
        return Err(CalcError::RunHoursExceed);
    }
    if i.spill_h > i.hours {
        return Err(CalcError::SpillHoursExceed);
    }
    let v_start = t.volume(i.z_start)? * 1e6;
    let v_end = t.volume(i.z_end)? * 1e6;
    let delta_v = v_end - v_start;
    let v_turbine = (i.h1 * cfg.q_h1 + i.h2 * cfg.q_h2) * 3600.0;
    let v_release = i.spill_q * i.spill_h * 3600.0 + i.eco_q * i.hours * 3600.0;
    let w_in = delta_v + v_turbine + v_release;
    Ok(Outputs { v_start, v_end, delta_v, v_turbine, v_release, q_in: w_in / (i.hours * 3600.0), w_in })
}

/// Lưu lượng về giữa hai mực nước đo liên tiếp: ΔV/Δt cộng xả môi trường `eco_q` (m³/s),
/// không tính tuabin và xả tràn. Là `compute` với H1 = H2 = 0.
pub fn inflow_between_levels(
    t: &VolumeTable,
    z_prev: f64,
    z_now: f64,
    minutes: f64,
    eco_q: f64,
) -> Result<Outputs, CalcError> {
    let i = Inputs { z_start: z_prev, z_end: z_now, hours: minutes / 60.0, h1: 0.0, h2: 0.0, spill_q: 0.0, spill_h: 0.0, eco_q };
    compute(t, &Turbine::default(), &i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inflow_with_unchanged_level_equals_eco_release() {
        let o = inflow_between_levels(&VolumeTable::builtin(), 160.0, 160.0, 15.0, 1.5).unwrap();
        assert!((o.q_in - 1.5).abs() < 1e-9);
    }

    #[test]
    fn inflow_adds_eco_release_to_storage_change() {
        let t = VolumeTable::builtin();
        let o = inflow_between_levels(&t, 160.0, 160.1, 15.0, 1.5).unwrap();
        let dv = t.volume(160.1).unwrap() * 1e6 - t.volume(160.0).unwrap() * 1e6;
        assert!((o.q_in - (dv / 900.0 + 1.5)).abs() < 1e-9);
        assert_eq!(o.v_turbine, 0.0);
    }

    #[test]
    fn inflow_matches_compute_with_quarter_hour() {
        let t = VolumeTable::builtin();
        let i = Inputs { z_start: 158.0, z_end: 157.9, hours: 0.5, h1: 0.0, h2: 0.0, spill_q: 0.0, spill_h: 0.0, eco_q: 1.5 };
        let want = compute(&t, &Turbine::default(), &i).unwrap();
        let got = inflow_between_levels(&t, 158.0, 157.9, 30.0, 1.5).unwrap();
        assert_eq!(got.q_in, want.q_in);
    }

    #[test]
    fn inflow_rejects_levels_outside_table_and_bad_minutes() {
        let t = VolumeTable::builtin();
        assert!(inflow_between_levels(&t, 148.9, 150.0, 15.0, 1.5).is_err());
        assert!(inflow_between_levels(&t, 150.0, 165.1, 15.0, 1.5).is_err());
        assert!(inflow_between_levels(&t, 150.0, 150.0, 0.0, 1.5).is_err());
    }

    fn inputs(z_start: f64, z_end: f64, hours: f64, h1: f64, h2: f64) -> Inputs {
        Inputs { z_start, z_end, hours, h1, h2, spill_q: 0.0, spill_h: 0.0, eco_q: 0.0 }
    }

    fn run(i: &Inputs) -> Result<Outputs, CalcError> {
        compute(&VolumeTable::builtin(), &Turbine::default(), i)
    }

    #[test]
    fn t1_matches_hand_calculation() {
        let o = run(&inputs(160.0, 160.1, 1.0, 1.0, 0.0)).unwrap();
        assert!((o.delta_v - 89_162.6).abs() < 1.0, "ΔV = {}", o.delta_v);
        assert!((o.q_in - 42.07).abs() < 0.01, "Q = {}", o.q_in);
    }

    #[test]
    fn t2_equal_levels_give_zero_delta() {
        let o = run(&inputs(155.0, 155.0, 2.0, 2.0, 2.0)).unwrap();
        assert_eq!(o.delta_v, 0.0);
        assert!((o.q_in - 34.6).abs() < 1e-9);
    }

    #[test]
    fn t4_start_at_top_row_is_ok() {
        let o = run(&inputs(165.0, 164.0, 1.0, 0.0, 0.0)).unwrap();
        assert!((o.v_start - 14_150_610.5).abs() < 1.0);
    }

    #[test]
    fn t5_levels_out_of_range_rejected() {
        assert!(matches!(
            run(&inputs(148.9, 160.0, 1.0, 0.0, 0.0)),
            Err(CalcError::LevelOutOfRange { .. })
        ));
        assert!(matches!(
            run(&inputs(160.0, 165.1, 1.0, 0.0, 0.0)),
            Err(CalcError::LevelOutOfRange { .. })
        ));
    }

    #[test]
    fn t6_zero_hours_rejected() {
        assert_eq!(run(&inputs(160.0, 160.1, 0.0, 0.0, 0.0)).err(), Some(CalcError::HoursNotPositive));
    }

    #[test]
    fn spill_and_min_flow_add_to_inflow() {
        let mut i = inputs(155.0, 155.0, 2.0, 0.0, 0.0);
        i.spill_q = 10.0;
        i.spill_h = 1.0;
        i.eco_q = 1.0;
        let o = run(&i).unwrap();
        assert_eq!(o.v_release, 10.0 * 3600.0 + 1.0 * 2.0 * 3600.0);
        assert!((o.q_in - (36_000.0 + 7_200.0) / 7_200.0).abs() < 1e-9);
        assert_eq!(o.w_in, o.delta_v + o.v_turbine + o.v_release);
    }

    #[test]
    fn run_hours_cannot_exceed_calculation_time() {
        assert_eq!(run(&inputs(155.0, 155.0, 2.0, 2.5, 0.0)).err(), Some(CalcError::RunHoursExceed));
        assert_eq!(run(&inputs(155.0, 155.0, 2.0, 0.0, 2.5)).err(), Some(CalcError::RunHoursExceed));
    }

    #[test]
    fn spill_hours_cannot_exceed_calculation_time() {
        let mut i = inputs(155.0, 155.0, 2.0, 0.0, 0.0);
        i.spill_h = 3.0;
        assert_eq!(run(&i).err(), Some(CalcError::SpillHoursExceed));
    }

    #[test]
    fn negative_or_non_finite_values_rejected() {
        let mut i = inputs(155.0, 155.0, 2.0, 0.0, 0.0);
        i.eco_q = -1.0;
        assert_eq!(run(&i).err(), Some(CalcError::Negative));
        assert_eq!(run(&inputs(155.0, 155.0, 2.0, -1.0, 0.0)).err(), Some(CalcError::Negative));
        assert_eq!(run(&inputs(f64::NAN, 155.0, 2.0, 0.0, 0.0)).err(), Some(CalcError::InvalidNumber));
        assert_eq!(run(&inputs(155.0, 155.0, f64::INFINITY, 0.0, 0.0)).err(), Some(CalcError::InvalidNumber));
    }
}
