// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use crate::data::properties::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LayerType {
    Dielectric,
    Conductor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DielectricLayer {
    pub name: String,
    pub thickness: f64,
    pub dielectric_constant: f64,
    pub measured_from: Option<String>,
    pub sw_t: Option<f64>,
    pub tw_t: Option<f64>,
    pub z_position: f64,
    pub auto_created: bool,
}

impl DielectricLayer {
    pub fn new(name: String, thickness: f64, dielectric_constant: f64) -> Self {
        Self {
            name,
            thickness,
            dielectric_constant,
            measured_from: None,
            sw_t: None,
            tw_t: None,
            z_position: 0.0,
            auto_created: false,
        }
    }

    pub fn new_auto_created(name: String, thickness: f64, dielectric_constant: f64) -> Self {
        Self {
            name,
            thickness,
            dielectric_constant,
            measured_from: None,
            sw_t: None,
            tw_t: None,
            z_position: 0.0,
            auto_created: true,
        }
    }

    pub fn with_position(mut self, z_position: f64) -> Self {
        self.z_position = z_position;
        self
    }

    pub fn with_measured_from(mut self, measured_from: String) -> Self {
        self.measured_from = Some(measured_from);
        self
    }

    pub fn get_layer_type(&self) -> LayerType {
        LayerType::Dielectric
    }

    pub fn get_top_z(&self) -> f64 {
        self.z_position + self.thickness
    }

    pub fn get_bottom_z(&self) -> f64 {
        self.z_position
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConductorLayer {
    pub name: String,
    pub thickness: f64,
    pub electrical_props: ElectricalProperties,
    pub physical_props: PhysicalProperties,
    pub rho_vs_width_spacing: Option<LookupTable2D>,
    pub rho_vs_si_width_thickness: Option<LookupTable2D>,
    pub etch_vs_width_spacing: Option<LookupTable2D>,
    pub etch_from_top: Option<LookupTable2D>,
    pub thickness_vs_width_spacing: Option<LookupTable2D>,
    pub crt_vs_si_width: Option<CrtVsSiWidthTable>,
    pub process_variation: Option<ProcessVariation>,
    pub resistive_only_etch: Option<f64>,
    pub capacitive_only_etch: Option<f64>,
    pub z_position: f64,
}

impl ConductorLayer {
    pub fn new(name: String, thickness: f64) -> Self {
        Self {
            name,
            thickness,
            electrical_props: ElectricalProperties {
                crt1: None,
                crt2: None,
                rpsq: None,
                rpv: None,
            },
            physical_props: PhysicalProperties {
                thickness,
                width_min: None,
                spacing_min: None,
                side_tangent: None,
                dielectric_constant: None,
            },
            rho_vs_width_spacing: None,
            rho_vs_si_width_thickness: None,
            etch_vs_width_spacing: None,
            etch_from_top: None,
            thickness_vs_width_spacing: None,
            crt_vs_si_width: None,
            process_variation: None,
            resistive_only_etch: None,
            capacitive_only_etch: None,
            z_position: 0.0,
        }
    }

    pub fn with_position(mut self, z_position: f64) -> Self {
        self.z_position = z_position;
        self
    }

    pub fn with_electrical_props(mut self, props: ElectricalProperties) -> Self {
        self.electrical_props = props;
        self
    }

    pub fn with_side_tangent(mut self, side_tangent: f64) -> Self {
        self.physical_props.side_tangent = Some(side_tangent);
        self
    }

    pub fn with_width_spacing_limits(mut self, wmin: f64, smin: f64) -> Self {
        self.physical_props.width_min = Some(wmin);
        self.physical_props.spacing_min = Some(smin);
        self
    }

    pub fn get_layer_type(&self) -> LayerType {
        LayerType::Conductor
    }

    pub fn get_top_z(&self) -> f64 {
        self.z_position + self.thickness
    }

    pub fn get_bottom_z(&self) -> f64 {
        self.z_position
    }

    pub fn is_trapezoid(&self) -> bool {
        self.physical_props.side_tangent.is_some()
    }

    pub fn get_trapezoid_angle(&self) -> f64 {
        self.physical_props
            .side_tangent
            .map(|tan_theta| tan_theta.atan())
            .unwrap_or(0.0)
    }

    pub fn calculate_resistance(
        &self,
        width: f64,
        length: f64,
        temperature: f64,
        reference_temp: f64,
    ) -> Option<f64> {
        println!("=== Resistance Calculation Debug ===");
        println!("Layer: {}", self.name);
        println!(
            "Width: {:.6} um, Length: {:.6} um, Thickness: {:.6} um",
            width, length, self.thickness
        );
        println!("Temperature: {temperature:.2}°C, Reference: {reference_temp:.2}°C");

        // Try to get RHO from different tables in priority order
        let (base_rho, rho_source) = if let Some(table) = &self.rho_vs_si_width_thickness {
            if let Some(rho) = table.lookup(width, self.thickness) {
                println!("Using RHO_VS_SI_WIDTH_AND_THICKNESS table lookup");
                println!("  Found rho = {rho:.6e} ohm*um (volume resistivity)");
                (rho, "RHO_VS_SI_WIDTH_AND_THICKNESS")
            } else {
                println!("RHO_VS_SI_WIDTH_AND_THICKNESS table lookup failed");
                return None;
            }
        } else if let Some(table) = &self.rho_vs_width_spacing {
            if let Some(rho) = table.lookup(width, 0.0) {
                println!("Using RHO_VS_WIDTH_SPACING table lookup");
                println!("  Found rho = {rho:.6e} ohm/sq (sheet resistance)");
                (rho, "RHO_VS_WIDTH_SPACING")
            } else {
                println!("RHO_VS_WIDTH_SPACING table lookup failed");
                return None;
            }
        } else if let Some(rpsq) = self.electrical_props.rpsq {
            println!("Using fixed RPSQ value");
            println!("  RPSQ = {rpsq:.6e} ohm/sq");
            (rpsq, "RPSQ")
        } else {
            println!("No resistivity data available");
            return None;
        };

        // Get CRT values from CRT_VS_SI_WIDTH table if available, otherwise use fixed values
        let (crt1, crt2) = if let Some(crt_table) = &self.crt_vs_si_width {
            if let Some((c1, c2)) = crt_table.lookup_crt_values(width) {
                println!("Using CRT_VS_SI_WIDTH table lookup");
                println!("  Interpolated CRT1 = {c1:.6e} /°C, CRT2 = {c2:.6e} /°C²");
                (c1, c2)
            } else {
                let c1 = self.electrical_props.crt1.unwrap_or(0.0);
                let c2 = self.electrical_props.crt2.unwrap_or(0.0);
                println!("CRT_VS_SI_WIDTH lookup failed, using fixed values");
                println!("  Fixed CRT1 = {c1:.6e} /°C, CRT2 = {c2:.6e} /°C²");
                (c1, c2)
            }
        } else {
            let c1 = self.electrical_props.crt1.unwrap_or(0.0);
            let c2 = self.electrical_props.crt2.unwrap_or(0.0);
            println!("Using fixed CRT values");
            println!("  Fixed CRT1 = {c1:.6e} /°C, CRT2 = {c2:.6e} /°C²");
            (c1, c2)
        };

        let temp_diff = temperature - reference_temp;
        let temp_coefficient = crt1 * temp_diff + crt2 * temp_diff.powi(2);
        println!("Temperature coefficient calculation:");
        println!("  ΔT = {temp_diff:.2}°C");
        println!("  Temp coefficient = CRT1*ΔT + CRT2*ΔT² = {temp_coefficient:.6e}");

        let temp_adjusted_rho = base_rho * (1.0 + temp_coefficient);
        println!("Temperature adjusted resistivity:");
        println!("  ρ(T) = ρ₀ * (1 + temp_coeff) = {temp_adjusted_rho:.6e}");

        // Calculate resistance based on resistivity type
        let resistance = if rho_source == "RHO_VS_SI_WIDTH_AND_THICKNESS" {
            // Volume resistivity formula: R = ρ * L / (W * T)
            let r = temp_adjusted_rho * length / (width * self.thickness);
            println!("Using volume resistivity formula:");
            println!(
                "  R = ρ * L / (W * T) = {:.6e} * {:.6} / ({:.6} * {:.6}) = {:.6e} Ω",
                temp_adjusted_rho, length, width, self.thickness, r
            );
            r
        } else {
            // Sheet resistance formula: R = Rsq * L / W
            let r = temp_adjusted_rho * length / width;
            println!("Using sheet resistance formula:");
            println!("  R = Rsq * L / W = {temp_adjusted_rho:.6e} * {length:.6} / {width:.6} = {r:.6e} Ω");
            r
        };

        println!("Final resistance: {resistance:.6e} Ω");
        println!("==============================");

        Some(resistance)
    }

    pub fn get_effective_width(&self, nominal_width: f64, spacing: f64) -> f64 {
        let etch_bias = self
            .etch_vs_width_spacing
            .as_ref()
            .and_then(|table| table.lookup(nominal_width, spacing))
            .unwrap_or(0.0);

        (nominal_width - 2.0 * etch_bias).max(0.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Layer {
    Dielectric(DielectricLayer),
    Conductor(Box<ConductorLayer>),
}

impl Layer {
    pub fn name(&self) -> &str {
        match self {
            Layer::Dielectric(layer) => &layer.name,
            Layer::Conductor(layer) => &layer.name,
        }
    }

    pub fn thickness(&self) -> f64 {
        match self {
            Layer::Dielectric(layer) => layer.thickness,
            Layer::Conductor(layer) => layer.thickness,
        }
    }

    pub fn z_position(&self) -> f64 {
        match self {
            Layer::Dielectric(layer) => layer.z_position,
            Layer::Conductor(layer) => layer.z_position,
        }
    }

    pub fn set_z_position(&mut self, z_position: f64) {
        match self {
            Layer::Dielectric(layer) => layer.z_position = z_position,
            Layer::Conductor(layer) => layer.z_position = z_position,
        }
    }

    pub fn get_top_z(&self) -> f64 {
        match self {
            Layer::Dielectric(layer) => layer.get_top_z(),
            Layer::Conductor(layer) => layer.get_top_z(),
        }
    }

    pub fn get_bottom_z(&self) -> f64 {
        match self {
            Layer::Dielectric(layer) => layer.get_bottom_z(),
            Layer::Conductor(layer) => layer.get_bottom_z(),
        }
    }

    pub fn layer_type(&self) -> LayerType {
        match self {
            Layer::Dielectric(_) => LayerType::Dielectric,
            Layer::Conductor(_) => LayerType::Conductor,
        }
    }

    pub fn is_conductor(&self) -> bool {
        matches!(self, Layer::Conductor(_))
    }

    pub fn is_dielectric(&self) -> bool {
        matches!(self, Layer::Dielectric(_))
    }

    pub fn is_auto_created(&self) -> bool {
        match self {
            Layer::Dielectric(layer) => layer.auto_created,
            Layer::Conductor(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_dielectric_layer_creation() {
        let layer =
            DielectricLayer::new("test_dielectric".to_string(), 1.0, 4.2).with_position(5.0);

        assert_eq!(layer.name, "test_dielectric");
        assert_eq!(layer.thickness, 1.0);
        assert_eq!(layer.dielectric_constant, 4.2);
        assert_eq!(layer.z_position, 5.0);
        assert_eq!(layer.get_top_z(), 6.0);
        assert_eq!(layer.get_bottom_z(), 5.0);
    }

    #[test]
    fn test_conductor_layer_creation() {
        let layer = ConductorLayer::new("metal1".to_string(), 0.5)
            .with_position(2.0)
            .with_side_tangent(0.1);

        assert_eq!(layer.name, "metal1");
        assert_eq!(layer.thickness, 0.5);
        assert_eq!(layer.z_position, 2.0);
        assert_eq!(layer.physical_props.side_tangent, Some(0.1));
        assert!(layer.is_trapezoid());
        assert_relative_eq!(layer.get_trapezoid_angle(), 0.1_f64.atan(), epsilon = 1e-10);
    }

    #[test]
    fn test_layer_enum() {
        let dielectric = Layer::Dielectric(DielectricLayer::new("test".to_string(), 1.0, 4.2));
        let conductor = Layer::Conductor(Box::new(ConductorLayer::new("metal".to_string(), 0.5)));

        assert!(dielectric.is_dielectric());
        assert!(!dielectric.is_conductor());
        assert!(conductor.is_conductor());
        assert!(!conductor.is_dielectric());

        assert_eq!(dielectric.name(), "test");
        assert_eq!(conductor.name(), "metal");
    }

    #[test]
    fn test_conductor_resistance_calculation() {
        let mut layer = ConductorLayer::new("metal1".to_string(), 0.2);
        layer.electrical_props.rpsq = Some(0.05);
        layer.electrical_props.crt1 = Some(0.003);
        layer.electrical_props.crt2 = Some(-1e-7);

        let resistance = layer.calculate_resistance(1.0, 10.0, 75.0, 25.0);
        assert!(resistance.is_some());

        let r = resistance.unwrap();
        assert!(r > 0.0);
    }

    #[test]
    fn test_effective_width_calculation() {
        let mut layer = ConductorLayer::new("metal1".to_string(), 0.2);

        let etch_table = LookupTable2D::new(
            vec![0.1, 0.2],
            vec![0.1, 0.2],
            vec![vec![0.01, 0.015], vec![0.005, 0.01]],
        );
        layer.etch_vs_width_spacing = Some(etch_table);

        let effective_width = layer.get_effective_width(0.2, 0.1);
        assert_relative_eq!(effective_width, 0.2 - 2.0 * 0.015, epsilon = 1e-10);
    }
}
