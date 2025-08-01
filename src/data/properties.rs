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
        let width_idx = self.find_index(&self.widths, width)?;
        let spacing_idx = self.find_index(&self.spacings, spacing)?;

        self.values.get(spacing_idx)?.get(width_idx).copied()
    }

    fn find_index(&self, array: &[f64], value: f64) -> Option<usize> {
        if array.is_empty() {
            return None;
        }

        if value <= array[0] {
            return Some(0);
        }

        if value >= array[array.len() - 1] {
            return Some(array.len() - 1);
        }

        for i in 0..array.len() - 1 {
            if value >= array[i] && value <= array[i + 1] {
                return if (value - array[i]).abs() < (value - array[i + 1]).abs() {
                    Some(i)
                } else {
                    Some(i + 1)
                };
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
}
