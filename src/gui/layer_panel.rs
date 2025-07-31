// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use egui::{Context, SidePanel, ScrollArea, CollapsingHeader, RichText, Color32};
use crate::data::{ProcessStack, Layer, LayerType};

pub struct LayerPanel {
    pub is_open: bool,
    pub selected_layer: Option<String>,
    pub show_electrical_props: bool,
    pub show_physical_props: bool,
    pub show_lookup_tables: bool,
}

impl LayerPanel {
    pub fn new() -> Self {
        Self {
            is_open: true,
            selected_layer: None,
            show_electrical_props: true,
            show_physical_props: true,
            show_lookup_tables: false,
        }
    }

    pub fn show(&mut self, ctx: &Context, stack: Option<&ProcessStack>) -> Option<String> {
        let mut layer_selected = None;
        
        if !self.is_open {
            return layer_selected;
        }

        SidePanel::left("layer_panel")
            .resizable(true)
            .default_width(300.0)
            .width_range(250.0..=500.0)
            .show(ctx, |ui| {
                ui.heading("Layer Information");
                
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.show_electrical_props, "Electrical");
                    ui.checkbox(&mut self.show_physical_props, "Physical");
                    ui.checkbox(&mut self.show_lookup_tables, "Tables");
                });
                
                ui.separator();

                if let Some(stack) = stack {
                    ScrollArea::vertical()
                        .id_source("layer_list")
                        .show(ui, |ui| {
                            self.show_process_summary(ui, stack);
                            ui.separator();
                            
                            self.show_layer_list(ui, stack, &mut layer_selected);
                            
                            ui.separator();
                            
                            if let Some(ref selected_name) = self.selected_layer {
                                if let Some(layer) = stack.get_layer(selected_name) {
                                    self.show_layer_details(ui, layer);
                                }
                            }
                        });
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("No ITF file loaded");
                    });
                }
            });

        layer_selected
    }

    fn show_process_summary(&self, ui: &mut egui::Ui, stack: &ProcessStack) {
        CollapsingHeader::new("Process Summary")
            .default_open(true)
            .show(ui, |ui| {
                let summary = stack.get_process_summary();
                
                ui.label(format!("Technology: {}", summary.technology_name));
                ui.label(format!("Total layers: {}", summary.total_layers));
                ui.label(format!("Conductors: {}", summary.conductor_layers));
                ui.label(format!("Dielectrics: {}", summary.dielectric_layers));
                ui.label(format!("Metal layers: {}", summary.metal_layers));
                ui.label(format!("Via connections: {}", summary.via_connections));
                
                if let Some(temp) = summary.global_temperature {
                    ui.label(format!("Temperature: {temp:.1}°C"));
                }
                
                ui.label(format!("Total height: {:.3} um", summary.total_height));
            });
    }

    fn show_layer_list(&mut self, ui: &mut egui::Ui, stack: &ProcessStack, layer_selected: &mut Option<String>) {
        CollapsingHeader::new("Layer Stack")
            .default_open(true)
            .show(ui, |ui| {
                // Show layers from top to bottom (reverse order)
                for layer in stack.layers.iter().rev() {
                    let is_selected = self.selected_layer.as_deref() == Some(layer.name());
                    
                    let layer_color = match layer.layer_type() {
                        LayerType::Conductor => Color32::from_rgb(255, 140, 0),
                        LayerType::Dielectric => Color32::from_rgb(100, 149, 237),
                    };
                    
                    let layer_icon = match layer.layer_type() {
                        LayerType::Conductor => "⚡",
                        LayerType::Dielectric => "▒",
                    };
                    
                    let layer_text = format!("{} {} ({:.3} um)", 
                        layer_icon, 
                        layer.name(), 
                        layer.thickness()
                    );
                    
                    let response = ui.selectable_label(
                        is_selected,
                        RichText::new(layer_text).color(layer_color)
                    );
                    
                    if response.clicked() {
                        if is_selected {
                            self.selected_layer = None;
                        } else {
                            self.selected_layer = Some(layer.name().to_string());
                            *layer_selected = Some(layer.name().to_string());
                        }
                    }
                }
            });
    }

    fn show_layer_details(&self, ui: &mut egui::Ui, layer: &Layer) {
        CollapsingHeader::new(format!("Layer Details: {}", layer.name()))
            .default_open(true)
            .show(ui, |ui| {
                // Basic properties
                ui.label(format!("Type: {:?}", layer.layer_type()));
                ui.label(format!("Thickness: {:.6} um", layer.thickness()));
                ui.label(format!("Z Position: {:.6} um", layer.z_position()));
                ui.label(format!("Bottom Z: {:.6} um", layer.get_bottom_z()));
                ui.label(format!("Top Z: {:.6} um", layer.get_top_z()));
                
                match layer {
                    Layer::Dielectric(d) => {
                        self.show_dielectric_details(ui, d);
                    }
                    Layer::Conductor(c) => {
                        self.show_conductor_details(ui, c);
                    }
                }
            });
    }

    fn show_dielectric_details(&self, ui: &mut egui::Ui, layer: &crate::data::DielectricLayer) {
        ui.separator();
        ui.label(RichText::new("Dielectric Properties").strong());
        
        ui.label(format!("Dielectric constant (ER): {:.2}", layer.dielectric_constant));
        
        if let Some(ref measured_from) = layer.measured_from {
            ui.label(format!("Measured from: {measured_from}"));
        }
        
        if let Some(sw_t) = layer.sw_t {
            ui.label(format!("SW_T: {sw_t:.6}"));
        }
        
        if let Some(tw_t) = layer.tw_t {
            ui.label(format!("TW_T: {tw_t:.6}"));
        }
    }

    fn show_conductor_details(&self, ui: &mut egui::Ui, layer: &crate::data::ConductorLayer) {
        if self.show_electrical_props {
            ui.separator();
            CollapsingHeader::new("Electrical Properties")
                .default_open(true)
                .show(ui, |ui| {
                    if let Some(crt1) = layer.electrical_props.crt1 {
                        ui.label(format!("CRT1: {crt1:.3e} /°C"));
                    }
                    
                    if let Some(crt2) = layer.electrical_props.crt2 {
                        ui.label(format!("CRT2: {crt2:.3e} /°C²"));
                    }
                    
                    if let Some(rpsq) = layer.electrical_props.rpsq {
                        ui.label(format!("Sheet resistance (RPSQ): {rpsq:.6} ohm/sq"));
                    }
                    
                    if let Some(rpv) = layer.electrical_props.rpv {
                        ui.label(format!("Resistance per via (RPV): {rpv:.3} ohm"));
                    }
                });
        }
        
        if self.show_physical_props {
            ui.separator();
            CollapsingHeader::new("Physical Properties")
                .default_open(true)
                .show(ui, |ui| {
                    if let Some(wmin) = layer.physical_props.width_min {
                        ui.label(format!("Min width (WMIN): {wmin:.6} um"));
                    }
                    
                    if let Some(smin) = layer.physical_props.spacing_min {
                        ui.label(format!("Min spacing (SMIN): {smin:.6} um"));
                    }
                    
                    if let Some(side_tangent) = layer.physical_props.side_tangent {
                        ui.label(format!("Side tangent: {side_tangent:.6}"));
                        let angle_deg = side_tangent.atan().to_degrees();
                        ui.label(format!("Side angle: {angle_deg:.2}°"));
                        
                        if side_tangent > 0.0 {
                            ui.label("Shape: Negative trapezoid (top wider)");
                        } else if side_tangent < 0.0 {
                            ui.label("Shape: Positive trapezoid (top narrower)");
                        } else {
                            ui.label("Shape: Rectangle");
                        }
                    }
                    
                    if let Some(resistive_etch) = layer.resistive_only_etch {
                        ui.label(format!("Resistive etch: {resistive_etch:.6} um"));
                    }
                    
                    if let Some(capacitive_etch) = layer.capacitive_only_etch {
                        ui.label(format!("Capacitive etch: {capacitive_etch:.6} um"));
                    }
                });
        }
        
        if self.show_lookup_tables {
            self.show_lookup_tables_info(ui, layer);
        }
    }

    fn show_lookup_tables_info(&self, ui: &mut egui::Ui, layer: &crate::data::ConductorLayer) {
        ui.separator();
        CollapsingHeader::new("Lookup Tables")
            .default_open(false)
            .show(ui, |ui| {
                if let Some(ref rho_table) = layer.rho_vs_width_spacing {
                    CollapsingHeader::new("Resistivity vs Width/Spacing")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.label(format!("Width points: {}", rho_table.widths.len()));
                            ui.label(format!("Spacing points: {}", rho_table.spacings.len()));
                            ui.label(format!("Value matrix: {}x{}", 
                                rho_table.values.len(),
                                rho_table.values.first().map(|row| row.len()).unwrap_or(0)
                            ));
                            
                            if !rho_table.widths.is_empty() {
                                ui.label(format!("Width range: {:.6} - {:.6} um", 
                                    rho_table.widths.first().unwrap_or(&0.0),
                                    rho_table.widths.last().unwrap_or(&0.0)
                                ));
                            }
                            
                            if !rho_table.spacings.is_empty() {
                                ui.label(format!("Spacing range: {:.6} - {:.6} um", 
                                    rho_table.spacings.first().unwrap_or(&0.0),
                                    rho_table.spacings.last().unwrap_or(&0.0)
                                ));
                            }
                        });
                }
                
                if let Some(ref etch_table) = layer.etch_vs_width_spacing {
                    CollapsingHeader::new("Etch vs Width/Spacing")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.label(format!("Width points: {}", etch_table.widths.len()));
                            ui.label(format!("Spacing points: {}", etch_table.spacings.len()));
                            ui.label(format!("Value matrix: {}x{}", 
                                etch_table.values.len(),
                                etch_table.values.first().map(|row| row.len()).unwrap_or(0)
                            ));
                        });
                }
                
                if let Some(ref thickness_table) = layer.thickness_vs_width_spacing {
                    CollapsingHeader::new("Thickness vs Width/Spacing")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.label(format!("Width points: {}", thickness_table.widths.len()));
                            ui.label(format!("Spacing points: {}", thickness_table.spacings.len()));
                            ui.label(format!("Value matrix: {}x{}", 
                                thickness_table.values.len(),
                                thickness_table.values.first().map(|row| row.len()).unwrap_or(0)
                            ));
                        });
                }
                
                if layer.rho_vs_width_spacing.is_none() && 
                   layer.etch_vs_width_spacing.is_none() &&
                   layer.thickness_vs_width_spacing.is_none() {
                    ui.label("No lookup tables available");
                }
            });
    }

    pub fn set_selected_layer(&mut self, layer_name: Option<String>) {
        self.selected_layer = layer_name;
    }

    pub fn get_selected_layer(&self) -> Option<&String> {
        self.selected_layer.as_ref()
    }

    pub fn toggle_visibility(&mut self) {
        self.is_open = !self.is_open;
    }
}

impl Default for LayerPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{TechnologyInfo, DielectricLayer, ConductorLayer};

    fn create_test_stack() -> ProcessStack {
        let tech = TechnologyInfo::new("test_stack".to_string());
        let mut stack = ProcessStack::new(tech);
        
        stack.add_layer(Layer::Dielectric(DielectricLayer::new("oxide1".to_string(), 1.0, 4.2)));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new("metal1".to_string(), 0.5))));
        
        stack
    }

    #[test]
    fn test_layer_panel_creation() {
        let panel = LayerPanel::new();
        assert!(panel.is_open);
        assert!(panel.selected_layer.is_none());
        assert!(panel.show_electrical_props);
        assert!(panel.show_physical_props);
        assert!(!panel.show_lookup_tables);
    }

    #[test]
    fn test_layer_selection() {
        let mut panel = LayerPanel::new();
        
        panel.set_selected_layer(Some("metal1".to_string()));
        assert_eq!(panel.get_selected_layer(), Some(&"metal1".to_string()));
        
        panel.set_selected_layer(None);
        assert_eq!(panel.get_selected_layer(), None);
    }

    #[test]
    fn test_panel_visibility() {
        let mut panel = LayerPanel::new();
        assert!(panel.is_open);
        
        panel.toggle_visibility();
        assert!(!panel.is_open);
        
        panel.toggle_visibility();
        assert!(panel.is_open);
    }

    #[test]
    fn test_property_display_flags() {
        let mut panel = LayerPanel::new();
        
        // Test initial state
        assert!(panel.show_electrical_props);
        assert!(panel.show_physical_props);
        assert!(!panel.show_lookup_tables);
        
        // Test toggles
        panel.show_electrical_props = false;
        panel.show_physical_props = false;
        panel.show_lookup_tables = true;
        
        assert!(!panel.show_electrical_props);
        assert!(!panel.show_physical_props);
        assert!(panel.show_lookup_tables);
    }
}