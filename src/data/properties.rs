// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectricalProperties {
    pub crt1: Option<f64>,
    pub crt2: Option<f64>,
    pub rpsq: Option<f64>,
    pub rpv: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysicalProperties {
    pub thickness: f64,
    pub width_min: Option<f64>,
    pub spacing_min: Option<f64>,
    pub side_tangent: Option<f64>,
    pub dielectric_constant: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LookupTable2D {
    pub widths: Vec<f64>,
    pub spacings: Vec<f64>,
    pub values: Vec<Vec<f64>>,
}

impl LookupTable2D {
    pub fn new(widths: Vec<f64>, spacings: Vec<f64>, values: Vec<Vec<f64>>) -> Self {
        Self {
            widths,
            spacings,
            values,
        }
    }

    pub fn lookup(&self, width: f64, spacing: f64) -> Option<f64> {
        if self.widths.is_empty() || self.spacings.is_empty() || self.values.is_empty() {
            return None;
        }

        // Find interpolation indices for width
        let (w_idx1, w_idx2, w_t) = self.find_interpolation_indices(&self.widths, width)?;

        // Find interpolation indices for spacing
        let (s_idx1, s_idx2, s_t) = self.find_interpolation_indices(&self.spacings, spacing)?;

        // Get the four corner values for bilinear interpolation
        let v11 = self.values.get(s_idx1)?.get(w_idx1).copied()?;
        let v12 = self.values.get(s_idx1)?.get(w_idx2).copied()?;
        let v21 = self.values.get(s_idx2)?.get(w_idx1).copied()?;
        let v22 = self.values.get(s_idx2)?.get(w_idx2).copied()?;

        // Bilinear interpolation
        let v1 = v11 + w_t * (v12 - v11); // Interpolate along width for spacing 1
        let v2 = v21 + w_t * (v22 - v21); // Interpolate along width for spacing 2
        let result = v1 + s_t * (v2 - v1); // Interpolate along spacing

        println!("2D Lookup interpolation debug:");
        println!(
            "  Width: {:.6} between indices {} ({:.6}) and {} ({:.6}), t={:.6}",
            width, w_idx1, self.widths[w_idx1], w_idx2, self.widths[w_idx2], w_t
        );
        println!(
            "  Spacing: {:.6} between indices {} ({:.6}) and {} ({:.6}), t={:.6}",
            spacing, s_idx1, self.spacings[s_idx1], s_idx2, self.spacings[s_idx2], s_t
        );
        println!("  Corner values: v11={v11:.6e}, v12={v12:.6e}, v21={v21:.6e}, v22={v22:.6e}");
        println!("  Interpolated result: {result:.6e}");

        Some(result)
    }

    fn find_interpolation_indices(&self, array: &[f64], value: f64) -> Option<(usize, usize, f64)> {
        if array.is_empty() {
            return None;
        }

        // Clamp to boundaries
        if value <= array[0] {
            return Some((0, 0, 0.0));
        }

        if value >= array[array.len() - 1] {
            let last_idx = array.len() - 1;
            return Some((last_idx, last_idx, 0.0));
        }

        // Find the interval containing the value
        for i in 0..array.len() - 1 {
            if value >= array[i] && value <= array[i + 1] {
                let t = if array[i + 1] != array[i] {
                    (value - array[i]) / (array[i + 1] - array[i])
                } else {
                    0.0
                };
                return Some((i, i + 1, t));
            }
        }

        None
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LookupTable1D {
    pub keys: Vec<f64>,
    pub values: Vec<f64>,
}

impl LookupTable1D {
    pub fn new(keys: Vec<f64>, values: Vec<f64>) -> Self {
        Self { keys, values }
    }

    pub fn lookup(&self, key: f64) -> Option<f64> {
        if self.keys.is_empty() {
            return None;
        }

        if key <= self.keys[0] {
            return self.values.first().copied();
        }

        if key >= self.keys[self.keys.len() - 1] {
            return self.values.get(self.keys.len() - 1).copied();
        }

        for i in 0..self.keys.len() - 1 {
            if key >= self.keys[i] && key <= self.keys[i + 1] {
                let t = (key - self.keys[i]) / (self.keys[i + 1] - self.keys[i]);
                let v1 = self.values.get(i)?;
                let v2 = self.values.get(i + 1)?;
                return Some(v1 + t * (v2 - v1));
            }
        }

        None
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrtVsSiWidthTable {
    pub widths: Vec<f64>,
    pub crt1_values: Vec<f64>,
    pub crt2_values: Vec<f64>,
}

impl CrtVsSiWidthTable {
    pub fn new(widths: Vec<f64>, crt1_values: Vec<f64>, crt2_values: Vec<f64>) -> Self {
        Self {
            widths,
            crt1_values,
            crt2_values,
        }
    }

    pub fn lookup_crt_values(&self, width: f64) -> Option<(f64, f64)> {
        if self.widths.is_empty() {
            return None;
        }

        // If width is less than smallest entry, use the first entry (no extrapolation)
        if width <= self.widths[0] {
            return Some((self.crt1_values[0], self.crt2_values[0]));
        }

        // If width is greater than largest entry, use the last entry (no extrapolation)
        if width >= self.widths[self.widths.len() - 1] {
            let last_idx = self.widths.len() - 1;
            return Some((self.crt1_values[last_idx], self.crt2_values[last_idx]));
        }

        // Linear interpolation between two points
        for i in 0..self.widths.len() - 1 {
            if width >= self.widths[i] && width <= self.widths[i + 1] {
                let t = (width - self.widths[i]) / (self.widths[i + 1] - self.widths[i]);
                let crt1 =
                    self.crt1_values[i] + t * (self.crt1_values[i + 1] - self.crt1_values[i]);
                let crt2 =
                    self.crt2_values[i] + t * (self.crt2_values[i + 1] - self.crt2_values[i]);
                return Some((crt1, crt2));
            }
        }

        None
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessVariation {
    pub density_polynomial_orders: Vec<u32>,
    pub width_polynomial_orders: Vec<u32>,
    pub width_ranges: Vec<f64>,
    pub polynomial_coefficients: Vec<Vec<f64>>,
}

impl ProcessVariation {
    pub fn calculate_thickness_variation(&self, density: f64, width: f64) -> f64 {
        let range_index = self.find_width_range(width);
        if range_index >= self.polynomial_coefficients.len() {
            return 0.0;
        }

        let coeffs = &self.polynomial_coefficients[range_index];
        let mut result = 0.0;
        let mut coeff_idx = 0;

        for &d_order in &self.density_polynomial_orders {
            for &w_order in &self.width_polynomial_orders {
                if coeff_idx < coeffs.len() {
                    result += coeffs[coeff_idx]
                        * density.powi(d_order as i32)
                        * width.powi(w_order as i32);
                    coeff_idx += 1;
                }
            }
        }

        result
    }

    fn find_width_range(&self, width: f64) -> usize {
        for (i, &range_limit) in self.width_ranges.iter().enumerate() {
            if width <= range_limit {
                return i;
            }
        }
        self.width_ranges.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_lookup_table_2d() {
        let table = LookupTable2D::new(
            vec![0.1, 0.2, 0.3],
            vec![0.05, 0.1, 0.15],
            vec![
                vec![1.0, 2.0, 3.0],
                vec![4.0, 5.0, 6.0],
                vec![7.0, 8.0, 9.0],
            ],
        );

        assert_eq!(table.lookup(0.1, 0.05), Some(1.0));
        assert_eq!(table.lookup(0.3, 0.15), Some(9.0));
        assert_eq!(table.lookup(0.2, 0.1), Some(5.0));
    }

    #[test]
    fn test_lookup_table_1d() {
        let table = LookupTable1D::new(vec![1.0, 2.0, 3.0], vec![10.0, 20.0, 30.0]);

        assert_eq!(table.lookup(1.0), Some(10.0));
        assert_eq!(table.lookup(3.0), Some(30.0));
        assert_relative_eq!(table.lookup(1.5).unwrap(), 15.0, epsilon = 1e-10);
    }

    #[test]
    fn test_process_variation() {
        let variation = ProcessVariation {
            density_polynomial_orders: vec![0, 1],
            width_polynomial_orders: vec![0, 1],
            width_ranges: vec![1.0, 2.0],
            polynomial_coefficients: vec![vec![1.0, 2.0, 3.0, 4.0], vec![5.0, 6.0, 7.0, 8.0]],
        };

        let result = variation.calculate_thickness_variation(0.5, 0.8);
        // Expected: coeffs[0]*density^0*width^0 + coeffs[1]*density^0*width^1 + coeffs[2]*density^1*width^0 + coeffs[3]*density^1*width^1
        // = 1.0*1*1 + 2.0*1*0.8 + 3.0*0.5*1 + 4.0*0.5*0.8 = 1.0 + 1.6 + 1.5 + 1.6 = 5.7
        assert_relative_eq!(result, 5.7, epsilon = 1e-10);
    }

    #[test]
    fn test_crt_vs_si_width_table() {
        let table = CrtVsSiWidthTable::new(
            vec![0.39, 0.45, 0.55, 0.70],
            vec![3.649e-3, 3.683e-3, 3.712e-3, 3.742e-3],
            vec![-8.535e-7, -8.532e-7, -8.247e-7, -8.902e-7],
        );

        // Test exact matches
        let result = table.lookup_crt_values(0.39).unwrap();
        assert_relative_eq!(result.0, 3.649e-3, epsilon = 1e-10);
        assert_relative_eq!(result.1, -8.535e-7, epsilon = 1e-10);

        // Test interpolation
        let result = table.lookup_crt_values(0.42).unwrap();
        // Should interpolate between 0.39 and 0.45
        let t = (0.42 - 0.39) / (0.45 - 0.39);
        let expected_crt1 = 3.649e-3 + t * (3.683e-3 - 3.649e-3);
        let expected_crt2 = -8.535e-7 + t * (-8.532e-7 - (-8.535e-7));
        assert_relative_eq!(result.0, expected_crt1, epsilon = 1e-10);
        assert_relative_eq!(result.1, expected_crt2, epsilon = 1e-10);

        // Test boundary conditions - below range
        let result = table.lookup_crt_values(0.30).unwrap();
        assert_relative_eq!(result.0, 3.649e-3, epsilon = 1e-10);
        assert_relative_eq!(result.1, -8.535e-7, epsilon = 1e-10);

        // Test boundary conditions - above range
        let result = table.lookup_crt_values(1.0).unwrap();
        assert_relative_eq!(result.0, 3.742e-3, epsilon = 1e-10);
        assert_relative_eq!(result.1, -8.902e-7, epsilon = 1e-10);
    }
}
