// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use crate::data::ProcessStack;

/// Thickness scaling system for exaggerated layer visualization
/// Maps actual thickness values to display thickness values using proportional scaling
/// where the thickest layer = 100% and thinnest layer = 30%
#[derive(Debug, Clone)]
pub struct ThicknessScaler {
    /// Minimum display thickness ratio (30%)
    pub min_ratio: f32,
    /// Maximum display thickness ratio (100%)
    pub max_ratio: f32,
    /// Actual thickness range from the stack
    thickness_range: Option<(f32, f32)>, // (min_thickness, max_thickness)
}

impl ThicknessScaler {
    /// Create a new thickness scaler with default ratios
    pub fn new() -> Self {
        Self {
            min_ratio: 0.3,  // 30%
            max_ratio: 1.0,  // 100%
            thickness_range: None,
        }
    }
    
    /// Create a thickness scaler with custom ratios
    pub fn new_with_ratios(min_ratio: f32, max_ratio: f32) -> Self {
        Self {
            min_ratio: min_ratio.clamp(0.1, 0.9),
            max_ratio: max_ratio.clamp(0.5, 1.0),
            thickness_range: None,
        }
    }
    
    /// Analyze a process stack to determine thickness range
    pub fn analyze_stack(&mut self, stack: &ProcessStack) {
        if stack.layers.is_empty() {
            self.thickness_range = None;
            return;
        }
        
        let mut min_thickness = f32::INFINITY;
        let mut max_thickness: f32 = 0.0;
        
        for layer in &stack.layers {
            let thickness = layer.thickness() as f32;
            if thickness > 0.0 {
                min_thickness = min_thickness.min(thickness);
                max_thickness = max_thickness.max(thickness);
            }
        }
        
        if min_thickness.is_finite() && max_thickness > min_thickness {
            self.thickness_range = Some((min_thickness, max_thickness));
        } else if min_thickness.is_finite() {
            // All layers have same thickness
            self.thickness_range = Some((min_thickness, min_thickness));
        } else {
            self.thickness_range = None;
        }
    }
    
    /// Get the exaggerated thickness for a given actual thickness
    pub fn get_exaggerated_thickness(&self, actual_thickness: f32) -> f32 {
        match self.thickness_range {
            Some((min_thick, max_thick)) if max_thick > min_thick => {
                // Proportional scaling between min_ratio and max_ratio
                let normalized = (actual_thickness - min_thick) / (max_thick - min_thick);
                let scale_factor = self.min_ratio + normalized * (self.max_ratio - self.min_ratio);
                actual_thickness * scale_factor
            }
            Some((_thickness, _)) => {
                // All layers same thickness, use max ratio
                actual_thickness * self.max_ratio
            }
            None => {
                // No valid thickness range, return original
                actual_thickness
            }
        }
    }
    
    /// Get the scaling factor for a given actual thickness
    pub fn get_scale_factor(&self, actual_thickness: f32) -> f32 {
        match self.thickness_range {
            Some((min_thick, max_thick)) if max_thick > min_thick => {
                let normalized = (actual_thickness - min_thick) / (max_thick - min_thick);
                self.min_ratio + normalized * (self.max_ratio - self.min_ratio)
            }
            Some(_) => self.max_ratio,
            None => 1.0,
        }
    }
    
    /// Get thickness statistics from the analyzed stack
    pub fn get_thickness_stats(&self) -> Option<ThicknessStats> {
        self.thickness_range.map(|(min_thick, max_thick)| ThicknessStats {
            min_thickness: min_thick,
            max_thickness: max_thick,
            thickness_ratio: if min_thick > 0.0 { max_thick / min_thick } else { 1.0 },
            min_scale_factor: self.min_ratio,
            max_scale_factor: self.max_ratio,
        })
    }
    
    /// Apply thickness exaggeration to all layers in a stack
    pub fn create_exaggerated_layer_heights(&self, stack: &ProcessStack) -> Vec<f32> {
        stack.layers.iter()
            .map(|layer| self.get_exaggerated_thickness(layer.thickness() as f32))
            .collect()
    }
    
    /// Get the total exaggerated height of the stack
    pub fn get_exaggerated_total_height(&self, stack: &ProcessStack) -> f32 {
        self.create_exaggerated_layer_heights(stack).iter().sum()
    }
}

impl Default for ThicknessScaler {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about thickness scaling
#[derive(Debug, Clone)]
pub struct ThicknessStats {
    pub min_thickness: f32,
    pub max_thickness: f32,
    pub thickness_ratio: f32,
    pub min_scale_factor: f32,
    pub max_scale_factor: f32,
}

impl ThicknessStats {
    /// Get a formatted description of the thickness scaling
    pub fn format_description(&self) -> String {
        format!(
            "Thickness range: {:.3}-{:.3} (ratio: {:.1}x), Scale factors: {:.0}%-{:.0}%",
            self.min_thickness,
            self.max_thickness,
            self.thickness_ratio,
            self.min_scale_factor * 100.0,
            self.max_scale_factor * 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{TechnologyInfo, DielectricLayer, ConductorLayer, Layer};

    fn create_test_stack_varied_thickness() -> ProcessStack {
        let tech = TechnologyInfo::new("test_varied".to_string());
        let mut stack = ProcessStack::new(tech);
        
        // Add layers with different thicknesses: 0.1, 0.5, 1.0, 2.0
        stack.add_layer(Layer::Dielectric(DielectricLayer::new("thin".to_string(), 0.1, 4.2)));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new("medium1".to_string(), 0.5))));
        stack.add_layer(Layer::Dielectric(DielectricLayer::new("medium2".to_string(), 1.0, 4.2)));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new("thick".to_string(), 2.0))));
        
        stack
    }

    fn create_test_stack_same_thickness() -> ProcessStack {
        let tech = TechnologyInfo::new("test_same".to_string());
        let mut stack = ProcessStack::new(tech);
        
        // All layers same thickness
        stack.add_layer(Layer::Dielectric(DielectricLayer::new("layer1".to_string(), 1.0, 4.2)));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new("layer2".to_string(), 1.0))));
        stack.add_layer(Layer::Dielectric(DielectricLayer::new("layer3".to_string(), 1.0, 4.2)));
        
        stack
    }

    #[test]
    fn test_thickness_scaler_creation() {
        let scaler = ThicknessScaler::new();
        assert_eq!(scaler.min_ratio, 0.3);
        assert_eq!(scaler.max_ratio, 1.0);
        assert!(scaler.thickness_range.is_none());
        
        let custom_scaler = ThicknessScaler::new_with_ratios(0.2, 0.8);
        assert_eq!(custom_scaler.min_ratio, 0.2);
        assert_eq!(custom_scaler.max_ratio, 0.8);
    }

    #[test]
    fn test_custom_ratio_clamping() {
        let scaler = ThicknessScaler::new_with_ratios(1.5, 0.1); // Invalid values
        assert!(scaler.min_ratio >= 0.1 && scaler.min_ratio <= 0.9);
        assert!(scaler.max_ratio >= 0.5 && scaler.max_ratio <= 1.0);
    }

    #[test]
    fn test_stack_analysis_varied_thickness() {
        let mut scaler = ThicknessScaler::new();
        let stack = create_test_stack_varied_thickness();
        
        scaler.analyze_stack(&stack);
        
        assert!(scaler.thickness_range.is_some());
        let (min_thick, max_thick) = scaler.thickness_range.unwrap();
        assert_eq!(min_thick, 0.1);
        assert_eq!(max_thick, 2.0);
    }

    #[test]
    fn test_stack_analysis_same_thickness() {
        let mut scaler = ThicknessScaler::new();
        let stack = create_test_stack_same_thickness();
        
        scaler.analyze_stack(&stack);
        
        assert!(scaler.thickness_range.is_some());
        let (min_thick, max_thick) = scaler.thickness_range.unwrap();
        assert_eq!(min_thick, 1.0);
        assert_eq!(max_thick, 1.0);
    }

    #[test]
    fn test_exaggerated_thickness_calculation() {
        let mut scaler = ThicknessScaler::new();
        let stack = create_test_stack_varied_thickness();
        scaler.analyze_stack(&stack);
        
        // Test boundary conditions
        let thin_exaggerated = scaler.get_exaggerated_thickness(0.1); // Thinnest
        let thick_exaggerated = scaler.get_exaggerated_thickness(2.0); // Thickest
        
        // Thinnest should be scaled to 30% of original
        assert!((thin_exaggerated - 0.1 * 0.3).abs() < 1e-6);
        
        // Thickest should be scaled to 100% of original
        assert!((thick_exaggerated - 2.0 * 1.0).abs() < 1e-6);
        
        // Middle values should be proportionally scaled
        let medium_exaggerated = scaler.get_exaggerated_thickness(1.0);
        assert!(medium_exaggerated > thin_exaggerated);
        assert!(medium_exaggerated < thick_exaggerated);
    }

    #[test]
    fn test_scale_factor_calculation() {
        let mut scaler = ThicknessScaler::new();
        let stack = create_test_stack_varied_thickness();
        scaler.analyze_stack(&stack);
        
        // Test scale factors
        assert_eq!(scaler.get_scale_factor(0.1), 0.3); // Minimum
        assert_eq!(scaler.get_scale_factor(2.0), 1.0); // Maximum
        
        // Middle value should be proportional
        let mid_factor = scaler.get_scale_factor(1.0);
        assert!(mid_factor > 0.3 && mid_factor < 1.0);
    }

    #[test]
    fn test_same_thickness_scaling() {
        let mut scaler = ThicknessScaler::new();
        let stack = create_test_stack_same_thickness();
        scaler.analyze_stack(&stack);
        
        // All layers same thickness should get max ratio
        let exaggerated = scaler.get_exaggerated_thickness(1.0);
        assert_eq!(exaggerated, 1.0); // 1.0 * max_ratio (1.0)
        
        let scale_factor = scaler.get_scale_factor(1.0);
        assert_eq!(scale_factor, 1.0);
    }

    #[test]
    fn test_empty_stack() {
        let mut scaler = ThicknessScaler::new();
        let tech = TechnologyInfo::new("empty".to_string());
        let stack = ProcessStack::new(tech);
        
        scaler.analyze_stack(&stack);
        assert!(scaler.thickness_range.is_none());
        
        // Should return original thickness when no range is set
        assert_eq!(scaler.get_exaggerated_thickness(1.0), 1.0);
        assert_eq!(scaler.get_scale_factor(1.0), 1.0);
    }

    #[test]
    fn test_thickness_stats() {
        let mut scaler = ThicknessScaler::new();
        let stack = create_test_stack_varied_thickness();
        scaler.analyze_stack(&stack);
        
        let stats = scaler.get_thickness_stats();
        assert!(stats.is_some());
        
        let stats = stats.unwrap();
        assert_eq!(stats.min_thickness, 0.1);
        assert_eq!(stats.max_thickness, 2.0);
        assert_eq!(stats.thickness_ratio, 20.0); // 2.0 / 0.1
        assert_eq!(stats.min_scale_factor, 0.3);
        assert_eq!(stats.max_scale_factor, 1.0);
        
        let description = stats.format_description();
        assert!(description.contains("0.100-2.000"));
        assert!(description.contains("30%-100%"));
    }

    #[test]
    fn test_exaggerated_layer_heights() {
        let mut scaler = ThicknessScaler::new();
        let stack = create_test_stack_varied_thickness();
        scaler.analyze_stack(&stack);
        
        let heights = scaler.create_exaggerated_layer_heights(&stack);
        assert_eq!(heights.len(), 4);
        
        // Should be in ascending order of exaggerated thickness
        // (though not necessarily same as original order due to scaling)
        assert!(heights[0] > 0.0); // thin layer scaled
        assert!(heights[3] > heights[0]); // thick layer should be larger
    }

    #[test]
    fn test_exaggerated_total_height() {
        let mut scaler = ThicknessScaler::new();
        let stack = create_test_stack_varied_thickness();
        scaler.analyze_stack(&stack);
        
        let _original_height = stack.get_total_height() as f32;
        let exaggerated_height = scaler.get_exaggerated_total_height(&stack);
        
        // Exaggerated height should be different from original
        // (unless all layers happen to get scale factor 1.0)
        assert!(exaggerated_height > 0.0);
        
        // Should equal sum of individual exaggerated heights
        let heights = scaler.create_exaggerated_layer_heights(&stack);
        let sum_heights: f32 = heights.iter().sum();
        assert!((exaggerated_height - sum_heights).abs() < 1e-6);
    }

    #[test]
    fn test_proportional_scaling() {
        let mut scaler = ThicknessScaler::new();
        let stack = create_test_stack_varied_thickness();
        scaler.analyze_stack(&stack);
        
        // Test that scaling is truly proportional
        // For thickness range 0.1 to 2.0, midpoint 1.05 should get mid scale factor
        let mid_thickness = 1.05; // Midpoint of 0.1 and 2.0
        let mid_scale = scaler.get_scale_factor(mid_thickness);
        let expected_mid = 0.3 + 0.5 * (1.0 - 0.3); // 0.65
        
        assert!((mid_scale - expected_mid).abs() < 0.01);
    }
}