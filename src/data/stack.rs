// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use serde::{Deserialize, Serialize};
use crate::data::{layer::Layer, via::ViaStack};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TechnologyInfo {
    pub name: String,
    pub global_temperature: Option<f64>,
    pub reference_direction: Option<String>,
    pub background_er: Option<f64>,
    pub half_node_scale_factor: Option<f64>,
    pub use_si_density: Option<bool>,
    pub drop_factor_lateral_spacing: Option<f64>,
}

impl TechnologyInfo {
    pub fn new(name: String) -> Self {
        Self {
            name,
            global_temperature: None,
            reference_direction: None,
            background_er: None,
            half_node_scale_factor: None,
            use_si_density: None,
            drop_factor_lateral_spacing: None,
        }
    }

    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.global_temperature = Some(temperature);
        self
    }

    pub fn with_reference_direction(mut self, direction: String) -> Self {
        self.reference_direction = Some(direction);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStack {
    pub technology_info: TechnologyInfo,
    pub layers: Vec<Layer>,
    pub via_stack: ViaStack,
    layer_name_to_index: HashMap<String, usize>,
    total_height: f64,
}

impl ProcessStack {
    pub fn new(technology_info: TechnologyInfo) -> Self {
        Self {
            technology_info,
            layers: Vec::new(),
            via_stack: ViaStack::new(),
            layer_name_to_index: HashMap::new(),
            total_height: 0.0,
        }
    }

    pub fn add_layer(&mut self, layer: Layer) {
        let layer_name = layer.name().to_string();
        let index = self.layers.len();
        
        self.layer_name_to_index.insert(layer_name, index);
        self.layers.push(layer);
        self.update_layer_positions();
    }

    pub fn add_via(&mut self, via: crate::data::via::ViaConnection) {
        self.via_stack.add_via(via);
        self.update_via_positions();
    }

    fn update_layer_positions(&mut self) {
        let mut current_z = 0.0;
        
        for layer in &mut self.layers {
            layer.set_z_position(current_z);
            current_z += layer.thickness();
        }
        
        self.total_height = current_z;
        self.update_via_positions();
    }

    fn update_via_positions(&mut self) {
        let layers = &self.layers;
        for via in &mut self.via_stack.vias {
            let from_layer = layers.iter().find(|l| l.name() == via.from_layer);
            let to_layer = layers.iter().find(|l| l.name() == via.to_layer);
            
            if let (Some(from_layer), Some(to_layer)) = (from_layer, to_layer) {
                let from_z = from_layer.get_top_z();
                let to_z = to_layer.get_bottom_z();
                
                let bottom_z = from_z.min(to_z);
                let top_z = from_z.max(to_z);
                
                via.z_position = bottom_z;
                via.height = top_z - bottom_z;
            }
        }
    }

    pub fn get_layer(&self, name: &str) -> Option<&Layer> {
        self.layer_name_to_index
            .get(name)
            .and_then(|&index| self.layers.get(index))
    }

    pub fn get_layer_mut(&mut self, name: &str) -> Option<&mut Layer> {
        self.layer_name_to_index
            .get(name)
            .and_then(|&index| self.layers.get_mut(index))
    }

    pub fn get_layer_by_index(&self, index: usize) -> Option<&Layer> {
        self.layers.get(index)
    }

    pub fn get_layers_in_z_range(&self, z_min: f64, z_max: f64) -> Vec<&Layer> {
        self.layers
            .iter()
            .filter(|layer| {
                let layer_bottom = layer.get_bottom_z();
                let layer_top = layer.get_top_z();
                
                layer_bottom < z_max && layer_top > z_min
            })
            .collect()
    }

    pub fn get_conductor_layers(&self) -> Vec<&Layer> {
        self.layers
            .iter()
            .filter(|layer| layer.is_conductor())
            .collect()
    }

    pub fn get_dielectric_layers(&self) -> Vec<&Layer> {
        self.layers
            .iter()
            .filter(|layer| layer.is_dielectric())
            .collect()
    }

    pub fn get_metal_layers(&self) -> Vec<&Layer> {
        self.layers
            .iter()
            .filter(|layer| {
                layer.is_conductor() && 
                (layer.name().starts_with("metal") || layer.name().starts_with("alpa"))
            })
            .collect()
    }

    pub fn get_total_height(&self) -> f64 {
        self.total_height
    }

    pub fn get_layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn get_conductor_count(&self) -> usize {
        self.get_conductor_layers().len()
    }

    pub fn get_dielectric_count(&self) -> usize {
        self.get_dielectric_layers().len()
    }

    pub fn get_via_count(&self) -> usize {
        self.via_stack.len()
    }

    pub fn validate_stack(&self) -> Result<(), StackValidationError> {
        if self.layers.is_empty() {
            return Err(StackValidationError::EmptyStack);
        }

        for (i, layer) in self.layers.iter().enumerate() {
            if layer.thickness() <= 0.0 {
                return Err(StackValidationError::InvalidThickness {
                    layer_name: layer.name().to_string(),
                    thickness: layer.thickness(),
                });
            }

            if i > 0 {
                let prev_layer = &self.layers[i - 1];
                let expected_z = prev_layer.get_top_z();
                let actual_z = layer.get_bottom_z();
                
                if (expected_z - actual_z).abs() > 1e-10 {
                    return Err(StackValidationError::LayerPositionMismatch {
                        layer_name: layer.name().to_string(),
                        expected_z,
                        actual_z,
                    });
                }
            }
        }

        for via in &self.via_stack.vias {
            if self.get_layer(&via.from_layer).is_none() {
                return Err(StackValidationError::UnknownLayer {
                    layer_name: via.from_layer.clone(),
                    via_name: via.name.clone(),
                });
            }
            
            if self.get_layer(&via.to_layer).is_none() {
                return Err(StackValidationError::UnknownLayer {
                    layer_name: via.to_layer.clone(),
                    via_name: via.name.clone(),
                });
            }
        }

        Ok(())
    }

    pub fn get_process_summary(&self) -> ProcessSummary {
        let metal_layers = self.get_metal_layers();
        let poly_layers: Vec<_> = self.layers
            .iter()
            .filter(|layer| layer.name().contains("poly"))
            .collect();

        ProcessSummary {
            technology_name: self.technology_info.name.clone(),
            total_layers: self.layers.len(),
            conductor_layers: self.get_conductor_count(),
            dielectric_layers: self.get_dielectric_count(),
            metal_layers: metal_layers.len(),
            poly_layers: poly_layers.len(),
            via_connections: self.via_stack.len(),
            total_height: self.total_height,
            global_temperature: self.technology_info.global_temperature,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSummary {
    pub technology_name: String,
    pub total_layers: usize,
    pub conductor_layers: usize,
    pub dielectric_layers: usize,
    pub metal_layers: usize,
    pub poly_layers: usize,
    pub via_connections: usize,
    pub total_height: f64,
    pub global_temperature: Option<f64>,
}

#[derive(Debug, thiserror::Error)]
pub enum StackValidationError {
    #[error("Stack is empty")]
    EmptyStack,
    
    #[error("Layer '{layer_name}' has invalid thickness: {thickness}")]
    InvalidThickness { layer_name: String, thickness: f64 },
    
    #[error("Layer '{layer_name}' position mismatch: expected {expected_z}, got {actual_z}")]
    LayerPositionMismatch { layer_name: String, expected_z: f64, actual_z: f64 },
    
    #[error("Via '{via_name}' references unknown layer '{layer_name}'")]
    UnknownLayer { layer_name: String, via_name: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{layer::*, via::ViaConnection};
    use approx::assert_relative_eq;

    #[test]
    fn test_technology_info_creation() {
        let tech = TechnologyInfo::new("test_tech".to_string())
            .with_temperature(25.0)
            .with_reference_direction("VERTICAL".to_string());
        
        assert_eq!(tech.name, "test_tech");
        assert_eq!(tech.global_temperature, Some(25.0));
        assert_eq!(tech.reference_direction, Some("VERTICAL".to_string()));
    }

    #[test]
    fn test_process_stack_creation() {
        let tech = TechnologyInfo::new("test_process".to_string());
        let stack = ProcessStack::new(tech);
        
        assert_eq!(stack.technology_info.name, "test_process");
        assert_eq!(stack.get_layer_count(), 0);
        assert_eq!(stack.get_total_height(), 0.0);
    }

    #[test]
    fn test_layer_addition_and_positioning() {
        let tech = TechnologyInfo::new("test_process".to_string());
        let mut stack = ProcessStack::new(tech);
        
        let dielectric1 = Layer::Dielectric(DielectricLayer::new("oxide1".to_string(), 1.0, 4.2));
        let conductor1 = Layer::Conductor(Box::new(ConductorLayer::new("metal1".to_string(), 0.5)));
        let dielectric2 = Layer::Dielectric(DielectricLayer::new("oxide2".to_string(), 2.0, 4.2));
        
        stack.add_layer(dielectric1);
        stack.add_layer(conductor1);
        stack.add_layer(dielectric2);
        
        assert_eq!(stack.get_layer_count(), 3);
        assert_relative_eq!(stack.get_total_height(), 3.5, epsilon = 1e-10);
        
        let layer1 = stack.get_layer("oxide1").unwrap();
        let layer2 = stack.get_layer("metal1").unwrap();
        let layer3 = stack.get_layer("oxide2").unwrap();
        
        assert_eq!(layer1.get_bottom_z(), 0.0);
        assert_eq!(layer1.get_top_z(), 1.0);
        assert_eq!(layer2.get_bottom_z(), 1.0);
        assert_eq!(layer2.get_top_z(), 1.5);
        assert_eq!(layer3.get_bottom_z(), 1.5);
        assert_eq!(layer3.get_top_z(), 3.5);
    }

    #[test]
    fn test_via_addition_and_positioning() {
        let tech = TechnologyInfo::new("test_process".to_string());
        let mut stack = ProcessStack::new(tech);
        
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new("metal1".to_string(), 0.5))));
        stack.add_layer(Layer::Dielectric(DielectricLayer::new("oxide".to_string(), 1.0, 4.2)));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new("metal2".to_string(), 0.5))));
        
        let via = ViaConnection::new(
            "via1".to_string(),
            "metal1".to_string(),
            "metal2".to_string(),
            0.04,
            5.0
        );
        
        stack.add_via(via);
        
        assert_eq!(stack.get_via_count(), 1);
        
        let via_ref = &stack.via_stack.vias[0];
        assert_eq!(via_ref.z_position, 0.5);
        assert_eq!(via_ref.height, 1.0);
        assert_eq!(via_ref.get_top_z(), 1.5);
        assert_eq!(via_ref.get_bottom_z(), 0.5);
    }

    #[test]
    fn test_layer_filtering() {
        let tech = TechnologyInfo::new("test_process".to_string());
        let mut stack = ProcessStack::new(tech);
        
        stack.add_layer(Layer::Dielectric(DielectricLayer::new("oxide1".to_string(), 1.0, 4.2)));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new("metal1".to_string(), 0.5))));
        stack.add_layer(Layer::Dielectric(DielectricLayer::new("oxide2".to_string(), 1.0, 4.2)));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new("poly".to_string(), 0.2))));
        
        assert_eq!(stack.get_conductor_count(), 2);
        assert_eq!(stack.get_dielectric_count(), 2);
        assert_eq!(stack.get_metal_layers().len(), 1);
        
        let layers_in_range = stack.get_layers_in_z_range(0.5, 2.0);
        assert_eq!(layers_in_range.len(), 3);
    }

    #[test]
    fn test_stack_validation() {
        let tech = TechnologyInfo::new("test_process".to_string());
        let mut stack = ProcessStack::new(tech);
        
        assert!(matches!(stack.validate_stack(), Err(StackValidationError::EmptyStack)));
        
        stack.add_layer(Layer::Dielectric(DielectricLayer::new("oxide1".to_string(), 1.0, 4.2)));
        assert!(stack.validate_stack().is_ok());
        
        let via = ViaConnection::new(
            "via1".to_string(),
            "unknown_layer".to_string(),
            "oxide1".to_string(),
            0.04,
            5.0
        );
        stack.add_via(via);
        
        assert!(matches!(stack.validate_stack(), Err(StackValidationError::UnknownLayer { .. })));
    }

    #[test]
    fn test_process_summary() {
        let tech = TechnologyInfo::new("test_1p3m".to_string()).with_temperature(85.0);
        let mut stack = ProcessStack::new(tech);
        
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new("poly".to_string(), 0.2))));
        stack.add_layer(Layer::Dielectric(DielectricLayer::new("oxide1".to_string(), 1.0, 4.2)));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new("metal1".to_string(), 0.5))));
        stack.add_layer(Layer::Dielectric(DielectricLayer::new("oxide2".to_string(), 1.0, 4.2)));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new("metal2".to_string(), 0.5))));
        
        let summary = stack.get_process_summary();
        
        assert_eq!(summary.technology_name, "test_1p3m");
        assert_eq!(summary.total_layers, 5);
        assert_eq!(summary.conductor_layers, 3);
        assert_eq!(summary.dielectric_layers, 2);
        assert_eq!(summary.metal_layers, 2);
        assert_eq!(summary.poly_layers, 1);
        assert_eq!(summary.global_temperature, Some(85.0));
        assert_relative_eq!(summary.total_height, 3.2, epsilon = 1e-10);
    }
}