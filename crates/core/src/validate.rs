use crate::error::CalcError;

/// Đọc số kiểu Việt: chấp nhận `160,5` và `160.5`; chỉ một dấu thập phân.
pub fn parse_vn_number(s: &str) -> Result<f64, CalcError> {
    let s = s.trim();
    let body = s.strip_prefix('-').unwrap_or(s);
    let only_number_chars = body.chars().all(|c| c.is_ascii_digit() || c == ',' || c == '.');
    if !only_number_chars || !body.chars().any(|c| c.is_ascii_digit()) {
        return Err(CalcError::InvalidNumber);
    }
    if body.chars().filter(|c| matches!(c, ',' | '.')).count() > 1 {
        return Err(CalcError::AmbiguousNumber);
    }
    s.replace(',', ".").parse().map_err(|_| CalcError::InvalidNumber)
}

/// Như `parse_vn_number` nhưng ô trống coi là 0 (ô H1, H2, xả).
pub fn parse_or_zero(s: &str) -> Result<f64, CalcError> {
    if s.trim().is_empty() {
        Ok(0.0)
    } else {
        parse_vn_number(s)
    }
}

/// Định dạng kiểu Việt: `799.802,19`.
pub fn format_vn(x: f64, decimals: usize) -> String {
    let fixed = format!("{x:.decimals$}");
    let (int_part, frac) = match fixed.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (fixed.as_str(), None),
    };
    let digits = int_part.trim_start_matches('-');
    let negative = int_part.starts_with('-') && fixed.chars().any(|c| matches!(c, '1'..='9'));
    let mut grouped = String::new();
    for (n, c) in digits.chars().enumerate() {
        if n > 0 && (digits.len() - n) % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(c);
    }
    let sign = if negative { "-" } else { "" };
    match frac {
        Some(f) => format!("{sign}{grouped},{f}"),
        None => format!("{sign}{grouped}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_comma_and_dot_decimals() {
        assert_eq!(parse_vn_number("160,5"), Ok(160.5));
        assert_eq!(parse_vn_number("160.5"), Ok(160.5));
        assert_eq!(parse_vn_number("  160,5 "), Ok(160.5));
        assert_eq!(parse_vn_number("-1,5"), Ok(-1.5));
        assert_eq!(parse_vn_number("2"), Ok(2.0));
    }

    #[test]
    fn rejects_text_empty_and_scientific() {
        for s in ["", "   ", "abc", "1e3", "NaN", "inf", "-", ",", "1,5m"] {
            assert_eq!(parse_vn_number(s), Err(CalcError::InvalidNumber), "{s:?}");
        }
    }

    #[test]
    fn rejects_two_separators_instead_of_guessing() {
        for s in ["1.234,5", "1,234.5", "1.234.567", "1,2,3"] {
            assert_eq!(parse_vn_number(s), Err(CalcError::AmbiguousNumber), "{s:?}");
        }
    }

    #[test]
    fn blank_is_zero_only_for_optional_fields() {
        assert_eq!(parse_or_zero(""), Ok(0.0));
        assert_eq!(parse_or_zero("  "), Ok(0.0));
        assert_eq!(parse_or_zero("2,5"), Ok(2.5));
        assert_eq!(parse_or_zero("x"), Err(CalcError::InvalidNumber));
    }

    #[test]
    fn formats_vietnamese_style() {
        assert_eq!(format_vn(799802.19, 2), "799.802,19");
        assert_eq!(format_vn(222.17, 2), "222,17");
        assert_eq!(format_vn(0.5, 2), "0,50");
        assert_eq!(format_vn(-1234.5, 2), "-1.234,50");
        assert_eq!(format_vn(165.0, 1), "165,0");
        assert_eq!(format_vn(1000000.0, 0), "1.000.000");
    }
}
