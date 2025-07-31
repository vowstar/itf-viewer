// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use egui::Color32;
use crate::data::{Layer, LayerType, ViaType};

pub struct ColorScheme {
    pub conductor_base: Color32,
    pub dielectric_base: Color32,
    pub via_metal: Color32,
    pub via_contact: Color32,
    pub substrate: Color32,
    pub poly: Color32,
    pub metal_colors: Vec<Color32>,
    pub selection_highlight: Color32,
    pub text_color: Color32,
    pub background: Color32,
}

impl ColorScheme {
    pub fn new() -> Self {
        Self {
            // Copper/orange tones for conductors and vias
            conductor_base: Color32::from_rgb(205, 127, 50),     // Bronze/copper
            via_metal: Color32::from_rgb(255, 140, 0),           // Dark orange
            via_contact: Color32::from_rgb(255, 165, 0),         // Orange
            
            // Blue/gray tones for dielectrics (silicon dioxide)
            dielectric_base: Color32::from_rgb(100, 149, 237),   // Cornflower blue
            substrate: Color32::from_rgb(47, 79, 79),            // Dark slate gray
            
            // Special conductor colors
            poly: Color32::from_rgb(255, 215, 0),                // Gold for polysilicon
            
            // Metal layer progression (orange to red tones)
            metal_colors: vec![
                Color32::from_rgb(255, 165, 0),   // M1: Orange
                Color32::from_rgb(255, 140, 0),   // M2: Dark orange  
                Color32::from_rgb(255, 99, 71),   // M3: Tomato
                Color32::from_rgb(220, 20, 60),   // M4: Crimson
                Color32::from_rgb(178, 34, 34),   // M5: Fire brick
                Color32::from_rgb(139, 0, 0),     // M6: Dark red
                Color32::from_rgb(128, 0, 0),     // M7: Maroon
                Color32::from_rgb(160, 82, 45),   // Top metals: Saddle brown
            ],
            
            // UI colors
            selection_highlight: Color32::from_rgb(255, 255, 0), // Yellow
            text_color: Color32::WHITE,
            background: Color32::from_rgb(25, 25, 25),           // Dark gray
        }
    }

    pub fn get_layer_color(&self, layer: &Layer, layer_index: usize) -> Color32 {
        match layer {
            Layer::Dielectric(d) => {
                if d.name.to_lowercase().contains("substrate") {
                    self.substrate
                } else {
                    self.get_dielectric_color(&d.name)
                }
            }
            Layer::Conductor(c) => {
                if c.name.to_lowercase().contains("poly") {
                    self.poly
                } else if c.name.to_lowercase().starts_with("metal") || 
                         c.name.to_lowercase().starts_with("alpa") {
                    self.get_metal_color(&c.name, layer_index)
                } else {
                    self.conductor_base
                }
            }
        }
    }

    pub fn get_via_color(&self, via_type: ViaType) -> Color32 {
        match via_type {
            ViaType::Contact => self.via_contact,
            ViaType::Metal => self.via_metal,
            ViaType::Other => self.conductor_base,
        }
    }

    fn get_dielectric_color(&self, layer_name: &str) -> Color32 {
        let name_lower = layer_name.to_lowercase();
        
        if name_lower.contains("nitride") {
            Color32::from_rgb(70, 130, 180)  // Steel blue for nitride
        } else if name_lower.contains("oxide") {
            Color32::from_rgb(100, 149, 237) // Cornflower blue for oxide
        } else if name_lower.contains("pass") {
            Color32::from_rgb(60, 100, 120)  // Darker blue for passivation
        } else if name_lower.contains("pmd") || name_lower.contains("imd") || name_lower.contains("ild") {
            Color32::from_rgb(90, 130, 200)  // Light blue for inter-metal dielectric
        } else {
            self.dielectric_base
        }
    }

    fn get_metal_color(&self, layer_name: &str, layer_index: usize) -> Color32 {
        let name_lower = layer_name.to_lowercase();
        
        // Extract metal number from name
        if let Some(metal_num) = self.extract_metal_number(&name_lower) {
            let color_index = (metal_num - 1).min(self.metal_colors.len() - 1);
            self.metal_colors[color_index]
        } else if name_lower.contains("alpa") || name_lower.contains("top") {
            // Top level thick metals
            *self.metal_colors.last().unwrap_or(&self.conductor_base)
        } else {
            // Fallback based on layer index
            let color_index = layer_index % self.metal_colors.len();
            self.metal_colors[color_index]
        }
    }

    fn extract_metal_number(&self, layer_name: &str) -> Option<usize> {
        // Try to extract number from patterns like "metal1", "m1", etc.
        if let Some(start) = layer_name.find("metal") {
            let after_metal = &layer_name[start + 5..];
            if let Ok(num) = after_metal.chars().take_while(|c| c.is_numeric()).collect::<String>().parse::<usize>() {
                return Some(num);
            }
        }
        
        if layer_name.starts_with('m') && layer_name.len() > 1 {
            if let Ok(num) = layer_name[1..].chars().take_while(|c| c.is_numeric()).collect::<String>().parse::<usize>() {
                return Some(num);
            }
        }
        
        None
    }

    pub fn get_layer_alpha(&self, layer: &Layer, is_selected: bool) -> u8 {
        let base_alpha = match layer.layer_type() {
            LayerType::Conductor => 220,
            LayerType::Dielectric => 100,
        };
        
        if is_selected {
            255 // Fully opaque when selected
        } else {
            base_alpha
        }
    }

    pub fn apply_alpha(&self, color: Color32, alpha: u8) -> Color32 {
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
    }

    pub fn get_dimension_text_color(&self) -> Color32 {
        Color32::LIGHT_GRAY
    }

    pub fn get_layer_outline_color(&self, is_selected: bool) -> Color32 {
        if is_selected {
            self.selection_highlight
        } else {
            Color32::from_gray(64)
        }
    }
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{DielectricLayer, ConductorLayer};

    #[test]
    fn test_dielectric_colors() {
        let scheme = ColorScheme::new();
        
        let oxide = Layer::Dielectric(DielectricLayer::new("oxide1".to_string(), 1.0, 4.2));
        let nitride = Layer::Dielectric(DielectricLayer::new("nitride".to_string(), 0.5, 7.0));
        let substrate = Layer::Dielectric(DielectricLayer::new("substrate".to_string(), 10.0, 11.9));
        
        let oxide_color = scheme.get_layer_color(&oxide, 0);
        let nitride_color = scheme.get_layer_color(&nitride, 1);
        let substrate_color = scheme.get_layer_color(&substrate, 2);
        
        // Colors should be different
        assert_ne!(oxide_color, nitride_color);
        assert_ne!(oxide_color, substrate_color);
        assert_ne!(nitride_color, substrate_color);
        
        // Substrate should use special color
        assert_eq!(substrate_color, scheme.substrate);
    }

    #[test]
    fn test_conductor_colors() {
        let scheme = ColorScheme::new();
        
        let poly = Layer::Conductor(Box::new(ConductorLayer::new("poly".to_string(), 0.2)));
        let metal1 = Layer::Conductor(Box::new(ConductorLayer::new("metal1".to_string(), 0.3)));
        let metal2 = Layer::Conductor(Box::new(ConductorLayer::new("metal2".to_string(), 0.4)));
        let alpa = Layer::Conductor(Box::new(ConductorLayer::new("alpa".to_string(), 2.0)));
        
        let poly_color = scheme.get_layer_color(&poly, 0);
        let metal1_color = scheme.get_layer_color(&metal1, 1);
        let metal2_color = scheme.get_layer_color(&metal2, 2);
        let alpa_color = scheme.get_layer_color(&alpa, 3);
        
        // Poly should use special color
        assert_eq!(poly_color, scheme.poly);
        
        // Metals should use progression
        assert_eq!(metal1_color, scheme.metal_colors[0]);
        assert_eq!(metal2_color, scheme.metal_colors[1]);
        
        // Alpa should use top metal color
        assert_eq!(alpa_color, *scheme.metal_colors.last().unwrap());
    }

    #[test]
    fn test_metal_number_extraction() {
        let scheme = ColorScheme::new();
        
        assert_eq!(scheme.extract_metal_number("metal1"), Some(1));
        assert_eq!(scheme.extract_metal_number("metal10"), Some(10));
        assert_eq!(scheme.extract_metal_number("m3"), Some(3));
        assert_eq!(scheme.extract_metal_number("m7_thick"), Some(7));
        assert_eq!(scheme.extract_metal_number("poly"), None);
        assert_eq!(scheme.extract_metal_number("alpa"), None);
    }

    #[test]
    fn test_via_colors() {
        let scheme = ColorScheme::new();
        
        let contact_color = scheme.get_via_color(ViaType::Contact);
        let metal_color = scheme.get_via_color(ViaType::Metal);
        let other_color = scheme.get_via_color(ViaType::Other);
        
        assert_eq!(contact_color, scheme.via_contact);
        assert_eq!(metal_color, scheme.via_metal);
        assert_eq!(other_color, scheme.conductor_base);
    }

    #[test]
    fn test_alpha_application() {
        let scheme = ColorScheme::new();
        let base_color = Color32::from_rgb(255, 128, 0);
        
        let transparent = scheme.apply_alpha(base_color, 128);
        let opaque = scheme.apply_alpha(base_color, 255);
        
        // Color32::from_rgba_unmultiplied may do premultiplication internally with egui
        // When alpha < 255, RGB values get premultiplied for GPU rendering
        // So we only test that alpha was applied correctly
        assert_eq!(transparent.a(), 128);
        assert_eq!(opaque.a(), 255);
        
        // For opaque colors, RGB should remain unchanged
        assert_eq!(opaque.r(), 255);
        assert_eq!(opaque.g(), 128);
        assert_eq!(opaque.b(), 0);
    }

    #[test]
    fn test_layer_alpha() {
        let scheme = ColorScheme::new();
        
        let conductor = Layer::Conductor(Box::new(ConductorLayer::new("metal1".to_string(), 0.3)));
        let dielectric = Layer::Dielectric(DielectricLayer::new("oxide".to_string(), 1.0, 4.2));
        
        let conductor_alpha = scheme.get_layer_alpha(&conductor, false);
        let dielectric_alpha = scheme.get_layer_alpha(&dielectric, false);
        let selected_alpha = scheme.get_layer_alpha(&conductor, true);
        
        assert_eq!(conductor_alpha, 220);
        assert_eq!(dielectric_alpha, 100);
        assert_eq!(selected_alpha, 255);
    }
}