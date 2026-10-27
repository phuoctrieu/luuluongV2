use crate::error::CalcError;

/// Bảng cao trình (m) – dung tích (triệu m³), sắp tăng dần.
pub struct VolumeTable {
    z: Vec<f64>,
    v: Vec<f64>,
}

impl VolumeTable {
    /// Bảng mặc định nhúng sẵn trong file chạy.
    pub fn builtin() -> Self {
        Self::from_csv_str(include_str!("../assets/volume_table.csv"))
            .expect("bảng dung tích nhúng sẵn phải hợp lệ")
    }

    /// Nạp CSV 2 cột `cao_trinh_m,dung_tich_trieu_m3`; tự sắp tăng dần.
    pub fn from_csv_str(s: &str) -> Result<Self, CalcError> {
        let invalid = |line: usize, reason: &str| CalcError::InvalidTable { line, reason: reason.into() };
        let mut reader = csv::ReaderBuilder::new().from_reader(s.as_bytes());
        let mut rows: Vec<(usize, f64, f64)> = Vec::new();
        for record in reader.records() {
            let record = record.map_err(|e| {
                let line = e.position().map_or(0, |p| p.line() as usize);
                invalid(line, "dòng phải có đúng 2 cột: cao trình, dung tích")
            })?;
            let line = record.position().map_or(0, |p| p.line() as usize);
            let number = |i: usize| record.get(i).and_then(|x| x.trim().parse::<f64>().ok());
            match (number(0), number(1)) {
                (Some(z), Some(v)) if z.is_finite() && v.is_finite() => {
                    // Làm tròn cao trình để loại sai số số thực (160,1000000000001 -> 160,1).
                    rows.push((line, (z * 1e4).round() / 1e4, v));
                }
                _ => return Err(invalid(line, "giá trị không phải số")),
            }
        }
        if rows.is_empty() {
            return Err(invalid(1, "bảng không có dòng dữ liệu"));
        }
        rows.sort_by(|a, b| a.1.total_cmp(&b.1));
        for pair in rows.windows(2) {
            let (_, z0, v0) = pair[0];
            let (line, z1, v1) = pair[1];
            if z1 <= z0 {
                return Err(invalid(line, "cao trình bị trùng"));
            }
            if v1 <= v0 {
                return Err(invalid(line, "dung tích phải tăng theo cao trình"));
            }
        }
        Ok(Self { z: rows.iter().map(|r| r.1).collect(), v: rows.iter().map(|r| r.2).collect() })
    }

    pub fn len(&self) -> usize {
        self.z.len()
    }

    pub fn is_empty(&self) -> bool {
        self.z.is_empty()
    }

    pub fn z_min(&self) -> f64 {
        self.z[0]
    }

    pub fn z_max(&self) -> f64 {
        self.z[self.z.len() - 1]
    }

    /// Dung tích (triệu m³) tại cao trình `z`, nội suy tuyến tính, không ngoại suy.
    pub fn volume(&self, z: f64) -> Result<f64, CalcError> {
        if !(z >= self.z_min() && z <= self.z_max()) {
            return Err(CalcError::LevelOutOfRange { min: self.z_min(), max: self.z_max() });
        }
        let i = self.z.partition_point(|&x| x < z);
        if self.z[i] == z {
            return Ok(self.v[i]);
        }
        let (z0, z1, v0, v1) = (self.z[i - 1], self.z[i], self.v[i - 1], self.v[i]);
        Ok(v0 + (z - z0) * (v1 - v0) / (z1 - z0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn builtin_table_has_125_rows_from_149_to_165() {
        let t = VolumeTable::builtin();
        assert_eq!(t.len(), 125);
        assert_eq!(t.z_min(), 149.0);
        assert_eq!(t.z_max(), 165.0);
    }

    #[test]
    fn t4_top_row_is_readable() {
        let t = VolumeTable::builtin();
        assert!(close(t.volume(165.0).unwrap(), 14.1506105036993));
    }

    #[test]
    fn exact_table_point_returns_table_value() {
        let t = VolumeTable::builtin();
        assert!(close(t.volume(160.1).unwrap(), 10.24670981115168));
        assert!(close(t.volume(149.0).unwrap(), 3.250388443608512));
    }

    #[test]
    fn t3_midpoint_is_average_of_neighbours() {
        let t = VolumeTable::builtin();
        let expect = (t.volume(160.0).unwrap() + t.volume(160.1).unwrap()) / 2.0;
        assert!(close(t.volume(160.05).unwrap(), expect));
    }

    #[test]
    fn interpolates_across_irregular_step() {
        // Bảng nhảy 164 -> 162: 163 nằm đúng giữa.
        let t = VolumeTable::builtin();
        let expect = (t.volume(162.0).unwrap() + t.volume(164.0).unwrap()) / 2.0;
        assert!(close(t.volume(163.0).unwrap(), expect));
    }

    #[test]
    fn t5_rejects_outside_range_without_extrapolating() {
        let t = VolumeTable::builtin();
        let err = CalcError::LevelOutOfRange { min: 149.0, max: 165.0 };
        assert_eq!(t.volume(148.9), Err(err));
        assert_eq!(t.volume(165.1), Err(CalcError::LevelOutOfRange { min: 149.0, max: 165.0 }));
        assert!(t.volume(f64::NAN).is_err());
    }

    #[test]
    fn csv_given_descending_is_sorted() {
        let t = VolumeTable::from_csv_str("cao_trinh_m,dung_tich_trieu_m3\n2,5\n1,3\n").unwrap();
        assert_eq!((t.z_min(), t.z_max()), (1.0, 2.0));
        assert!(close(t.volume(1.5).unwrap(), 4.0));
    }

    #[test]
    fn csv_rejects_duplicate_level() {
        let e = VolumeTable::from_csv_str("cao_trinh_m,dung_tich_trieu_m3\n1,3\n1,4\n").err();
        assert!(matches!(e, Some(CalcError::InvalidTable { .. })));
    }

    #[test]
    fn csv_rejects_volume_not_increasing() {
        let e = VolumeTable::from_csv_str("cao_trinh_m,dung_tich_trieu_m3\n1,3\n2,3\n").err();
        assert!(matches!(e, Some(CalcError::InvalidTable { line: 3, .. })));
    }

    #[test]
    fn csv_rejects_missing_column_and_bad_number() {
        assert!(VolumeTable::from_csv_str("cao_trinh_m\n1\n").is_err());
        assert!(VolumeTable::from_csv_str("cao_trinh_m,dung_tich_trieu_m3\n1,abc\n").is_err());
        assert!(VolumeTable::from_csv_str("cao_trinh_m,dung_tich_trieu_m3\n").is_err());
    }
}
