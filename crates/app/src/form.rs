//! Biểu mẫu nhập liệu: chuỗi người dùng gõ -> `Inputs`, lỗi theo từng ô.

use luuluong_core::validate::{parse_or_zero, parse_vn_number};
use luuluong_core::{CalcError, Inputs, VolumeTable};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    ZStart,
    ZEnd,
    Hours,
    H1,
    H2,
    SpillQ,
    SpillH,
    EcoQ,
}

impl Field {
    pub const ALL: [Field; 8] = [
        Field::ZStart,
        Field::ZEnd,
        Field::Hours,
        Field::H1,
        Field::H2,
        Field::SpillQ,
        Field::SpillH,
        Field::EcoQ,
    ];

    fn index(self) -> usize {
        Self::ALL.iter().position(|f| *f == self).unwrap()
    }
}

#[derive(Default, Clone)]
pub struct Form {
    values: [String; 8],
}

impl Form {
    pub fn get(&self, f: Field) -> &str {
        &self.values[f.index()]
    }

    pub fn get_mut(&mut self, f: Field) -> &mut String {
        &mut self.values[f.index()]
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn is_empty(&self) -> bool {
        self.values.iter().all(|v| v.trim().is_empty())
    }

    /// Có ô nào khác `other` không (để biết kết quả cũ đã lỗi thời).
    pub fn differs(&self, other: &Form) -> bool {
        self.values != other.values
    }
}

/// Kiểm tra mọi ô; trả `Inputs` hoặc danh sách (ô, lỗi) theo thứ tự trên xuống.
pub fn validate(form: &Form, table: &VolumeTable) -> Result<Inputs, Vec<(Field, CalcError)>> {
    let mut errors = Vec::new();
    let mut parse = |f: Field, required: bool| {
        let raw = form.get(f);
        let parsed = if required { parse_vn_number(raw) } else { parse_or_zero(raw) };
        match parsed {
            Ok(x) if x.is_finite() => Some(x),
            Ok(_) => {
                errors.push((f, CalcError::InvalidNumber));
                None
            }
            Err(e) => {
                errors.push((f, e));
                None
            }
        }
    };
    let z_start = parse(Field::ZStart, true);
    let z_end = parse(Field::ZEnd, true);
    let hours = parse(Field::Hours, true);
    let h1 = parse(Field::H1, false);
    let h2 = parse(Field::H2, false);
    let spill_q = parse(Field::SpillQ, false);
    let spill_h = parse(Field::SpillH, false);
    let eco_q = parse(Field::EcoQ, false);

    let mut check = |f: Field, value: Option<f64>, rule: &dyn Fn(f64) -> Option<CalcError>| {
        if let Some(e) = value.and_then(rule) {
            errors.push((f, e));
        }
    };
    let level = |z: f64| table.volume(z).err();
    check(Field::ZStart, z_start, &level);
    check(Field::ZEnd, z_end, &level);
    check(Field::Hours, hours, &|h| (h <= 0.0).then_some(CalcError::HoursNotPositive));
    // Chỉ so với thời gian tính khi thời gian hợp lệ, tránh báo lỗi dây chuyền.
    let limit = hours.filter(|h| *h > 0.0);
    let run = |h: f64| match (h < 0.0, limit) {
        (true, _) => Some(CalcError::Negative),
        (_, Some(t)) if h > t => Some(CalcError::RunHoursExceed),
        _ => None,
    };
    check(Field::H1, h1, &run);
    check(Field::H2, h2, &run);
    let non_negative = |x: f64| (x < 0.0).then_some(CalcError::Negative);
    check(Field::SpillQ, spill_q, &non_negative);
    check(Field::SpillH, spill_h, &|h| match (h < 0.0, limit) {
        (true, _) => Some(CalcError::Negative),
        (_, Some(t)) if h > t => Some(CalcError::SpillHoursExceed),
        _ => None,
    });
    check(Field::EcoQ, eco_q, &non_negative);

    if !errors.is_empty() {
        errors.sort_by_key(|(f, _)| f.index());
        return Err(errors);
    }
    Ok(Inputs {
        z_start: z_start.unwrap(),
        z_end: z_end.unwrap(),
        hours: hours.unwrap(),
        h1: h1.unwrap(),
        h2: h2.unwrap(),
        spill_q: spill_q.unwrap(),
        spill_h: spill_h.unwrap(),
        eco_q: eco_q.unwrap(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form(vals: [&str; 8]) -> Form {
        let mut f = Form::default();
        for (field, v) in Field::ALL.iter().zip(vals) {
            *f.get_mut(*field) = v.to_string();
        }
        f
    }

    fn errors(vals: [&str; 8]) -> Vec<(Field, CalcError)> {
        validate(&form(vals), &VolumeTable::builtin()).unwrap_err()
    }

    #[test]
    fn valid_form_with_comma_decimals_and_blank_optionals() {
        let i = validate(&form(["160,0", "160.1", "1", "1", "", "", "", ""]), &VolumeTable::builtin()).unwrap();
        assert_eq!((i.z_start, i.z_end, i.hours, i.h1, i.h2), (160.0, 160.1, 1.0, 1.0, 0.0));
        assert_eq!((i.spill_q, i.spill_h, i.eco_q), (0.0, 0.0, 0.0));
    }

    #[test]
    fn required_blank_fields_ask_for_a_number() {
        let e = errors(["", "160", "", "", "", "", "", ""]);
        assert_eq!(e[0], (Field::ZStart, CalcError::InvalidNumber));
        assert_eq!(e[1], (Field::Hours, CalcError::InvalidNumber));
    }

    #[test]
    fn t5_out_of_range_level_flagged_on_its_own_field() {
        let e = errors(["148,9", "165,1", "1", "0", "0", "", "", ""]);
        assert_eq!(e.len(), 2);
        assert!(matches!(e[0], (Field::ZStart, CalcError::LevelOutOfRange { .. })));
        assert!(matches!(e[1], (Field::ZEnd, CalcError::LevelOutOfRange { .. })));
    }

    #[test]
    fn t6_zero_hours_flagged_on_hours() {
        let e = errors(["160", "160", "0", "0", "0", "", "", ""]);
        assert_eq!(e, vec![(Field::Hours, CalcError::HoursNotPositive)]);
    }

    #[test]
    fn run_hours_cannot_exceed_hours() {
        let e = errors(["160", "160", "2", "3", "2,5", "", "", ""]);
        assert_eq!(
            e,
            vec![(Field::H1, CalcError::RunHoursExceed), (Field::H2, CalcError::RunHoursExceed)]
        );
    }

    #[test]
    fn spill_and_eco_rules() {
        let e = errors(["160", "160", "2", "0", "0", "-1", "3", "-2"]);
        assert_eq!(
            e,
            vec![
                (Field::SpillQ, CalcError::Negative),
                (Field::SpillH, CalcError::SpillHoursExceed),
                (Field::EcoQ, CalcError::Negative),
            ]
        );
    }

    #[test]
    fn ambiguous_separator_reported_per_field() {
        let e = errors(["1.234,5", "160", "1", "0", "0", "", "", ""]);
        assert_eq!(e[0], (Field::ZStart, CalcError::AmbiguousNumber));
    }

    #[test]
    fn hours_dependent_checks_skipped_when_hours_invalid() {
        // Thời gian lỗi thì không báo thêm lỗi H1 > thời gian (tránh báo nhầm).
        let e = errors(["160", "160", "abc", "5", "0", "", "", ""]);
        assert_eq!(e, vec![(Field::Hours, CalcError::InvalidNumber)]);
    }

    #[test]
    fn clear_empties_every_field() {
        let mut f = form(["1", "2", "3", "4", "5", "6", "7", "8"]);
        assert!(!f.is_empty());
        f.clear();
        assert!(f.is_empty());
    }
}
