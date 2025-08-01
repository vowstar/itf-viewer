// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use crate::data::{LayerType, ProcessStack};
use egui::{CollapsingHeader, Color32, Context, RichText, ScrollArea, SidePanel};

pub struct LayerPanel {
    pub is_open: bool,
    pub selected_layer: Option<String>,
}

impl LayerPanel {
    pub fn new() -> Self {
        Self {
            is_open: true,
            selected_layer: None,
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
                ui.separator();

                if let Some(stack) = stack {
                    ScrollArea::vertical()
                        .id_source("layer_list")
                        .show(ui, |ui| {
                            self.show_process_summary(ui, stack);
                            ui.separator();

                            self.show_layer_list(ui, stack, &mut layer_selected);
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

    fn show_layer_list(
        &mut self,
        ui: &mut egui::Ui,
        stack: &ProcessStack,
        layer_selected: &mut Option<String>,
    ) {
        CollapsingHeader::new("Layer Stack")
            .default_open(true)
            .show(ui, |ui| {
                // Show layers from top to bottom (ITF order matches visual expectation)
                for layer in stack.layers.iter() {
                    let is_selected = self.selected_layer.as_deref() == Some(layer.name());

                    let layer_color = match layer.layer_type() {
                        LayerType::Conductor => Color32::from_rgb(255, 140, 0),
                        LayerType::Dielectric => Color32::from_rgb(100, 149, 237),
                    };

                    let layer_icon = match layer.layer_type() {
                        LayerType::Conductor => "C",
                        LayerType::Dielectric => "D",
                    };

                    let layer_text = format!(
                        "{} {} ({:.3} um)",
                        layer_icon,
                        layer.name(),
                        layer.thickness()
                    );

                    let response = ui.selectable_label(
                        is_selected,
                        RichText::new(layer_text).color(layer_color),
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

        // Show via connections
        if !stack.via_stack.is_empty() {
            CollapsingHeader::new("Via Connections")
                .default_open(true)
                .show(ui, |ui| {
                    for via in stack.via_stack.iter() {
                        let via_color = Color32::from_rgb(192, 192, 192);
                        let via_text = format!(
                            "V {} -> {} ({:.2} um^2)",
                            via.from_layer, via.to_layer, via.area
                        );

                        let is_selected = self.selected_layer.as_deref() == Some(&via.name);
                        let response = ui.selectable_label(
                            is_selected,
                            RichText::new(via_text).color(via_color),
                        );

                        if response.clicked() {
                            if is_selected {
                                self.selected_layer = None;
                            } else {
                                self.selected_layer = Some(via.name.clone());
                                *layer_selected = Some(via.name.clone());
                            }
                        }
                    }
                });
        }
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

    #[test]
    fn test_layer_panel_creation() {
        let panel = LayerPanel::new();
        assert!(panel.is_open);
        assert!(panel.selected_layer.is_none());
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
        let panel = LayerPanel::new();

        // Test initial state - no property flags anymore
        assert!(panel.is_open);
        assert!(panel.selected_layer.is_none());
    }
}
