use crate::error::CalcError;
use crate::reservoir::VolumeTable;

/// Lưu lượng mỗi tổ máy (m³/s).
pub struct Turbine {
    pub q_h1: f64,
    pub q_h2: f64,
}

impl Default for Turbine {
    fn default() -> Self {
        Self { q_h1: 17.3, q_h2: 17.3 }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

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
