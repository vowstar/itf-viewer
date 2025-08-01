// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use crate::data::{Layer, ProcessStack};
use egui::{CollapsingHeader, Context, RichText, ScrollArea, SidePanel};

pub struct LayerDetailsPanel {
    pub is_open: bool,
    pub selected_layer: Option<String>,
    pub show_electrical_props: bool,
    pub show_physical_props: bool,
    pub show_lookup_tables: bool,
}

impl LayerDetailsPanel {
    pub fn new() -> Self {
        Self {
            is_open: true,
            selected_layer: None,
            show_electrical_props: true,
            show_physical_props: true,
            show_lookup_tables: false,
        }
    }

    pub fn show(&mut self, ctx: &Context, stack: Option<&ProcessStack>) {
        if !self.is_open {
            return;
        }

        SidePanel::right("layer_details_panel")
            .resizable(true)
            .default_width(350.0)
            .width_range(300.0..=600.0)
            .show(ctx, |ui| {
                // Title with current layer name
                let title = if let Some(ref layer_name) = self.selected_layer {
                    format!("Layer Details: {layer_name}")
                } else {
                    "Layer Details: None".to_string()
                };
                ui.heading(title);

                // Property display options
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.show_electrical_props, "Electrical");
                    ui.checkbox(&mut self.show_physical_props, "Physical");
                    ui.checkbox(&mut self.show_lookup_tables, "Tables");
                });

                ui.separator();

                if let Some(stack) = stack {
                    if let Some(ref selected_name) = self.selected_layer {
                        ScrollArea::vertical()
                            .id_salt("layer_details_scroll")
                            .show(ui, |ui| {
                                if let Some(layer) = stack.get_layer(selected_name) {
                                    self.show_layer_details(ui, layer);
                                } else if let Some(via) =
                                    stack.via_stack.iter().find(|v| &v.name == selected_name)
                                {
                                    self.show_via_details(ui, via);
                                } else {
                                    ui.centered_and_justified(|ui| {
                                        ui.label("Selected layer not found");
                                    });
                                }
                            });
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label("No layer selected\nClick on a layer to view details");
                        });
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("No ITF file loaded");
                    });
                }
            });
    }

    fn show_layer_details(&self, ui: &mut egui::Ui, layer: &Layer) {
        // Basic properties
        CollapsingHeader::new("Basic Properties")
            .default_open(true)
            .show(ui, |ui| {
                ui.label(format!("Name: {}", layer.name()));
                ui.label(format!("Type: {:?}", layer.layer_type()));
                ui.label(format!("Thickness: {:.6} μm", layer.thickness()));
                ui.label(format!("Z Position: {:.6} μm", layer.z_position()));
                ui.label(format!("Bottom Z: {:.6} μm", layer.get_bottom_z()));
                ui.label(format!("Top Z: {:.6} μm", layer.get_top_z()));
            });

        match layer {
            Layer::Dielectric(d) => {
                self.show_dielectric_details(ui, d);
            }
            Layer::Conductor(c) => {
                self.show_conductor_details(ui, c);
            }
        }
    }

    fn show_dielectric_details(&self, ui: &mut egui::Ui, layer: &crate::data::DielectricLayer) {
        CollapsingHeader::new("Dielectric Properties")
            .default_open(true)
            .show(ui, |ui| {
                ui.label(format!(
                    "Dielectric constant (εᵣ): {:.2}",
                    layer.dielectric_constant
                ));

                if let Some(ref measured_from) = layer.measured_from {
                    ui.label(format!("Measured from: {measured_from}"));
                }

                if let Some(sw_t) = layer.sw_t {
                    ui.label(format!("SW_T: {sw_t:.6} μm"));
                }

                if let Some(tw_t) = layer.tw_t {
                    ui.label(format!("TW_T: {tw_t:.6} μm"));
                }
            });
    }

    fn show_conductor_details(&self, ui: &mut egui::Ui, layer: &crate::data::ConductorLayer) {
        if self.show_electrical_props {
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
                        ui.label(format!("Sheet resistance (RPSQ): {rpsq:.6} Ω/□"));
                    }

                    if let Some(rpv) = layer.electrical_props.rpv {
                        ui.label(format!("Resistance per via (RPV): {rpv:.3} Ω"));
                    }

                    if layer.electrical_props.crt1.is_none()
                        && layer.electrical_props.crt2.is_none()
                        && layer.electrical_props.rpsq.is_none()
                        && layer.electrical_props.rpv.is_none()
                    {
                        ui.label("No electrical properties available");
                    }
                });
        }

        if self.show_physical_props {
            CollapsingHeader::new("Physical Properties")
                .default_open(true)
                .show(ui, |ui| {
                    if let Some(wmin) = layer.physical_props.width_min {
                        ui.label(format!("Min width (WMIN): {wmin:.6} μm"));
                    }

                    if let Some(smin) = layer.physical_props.spacing_min {
                        ui.label(format!("Min spacing (SMIN): {smin:.6} μm"));
                    }

                    if let Some(side_tangent) = layer.physical_props.side_tangent {
                        ui.label(format!("Side tangent: {side_tangent:.6}"));
                        let angle_deg = side_tangent.atan().to_degrees();
                        ui.label(format!("Side angle: {angle_deg:.2}°"));

                        ui.separator();
                        if side_tangent > 0.0 {
                            ui.label(
                                RichText::new("Shape: Negative trapezoid")
                                    .color(egui::Color32::from_rgb(255, 165, 0)),
                            );
                            ui.label("(top wider than bottom)");
                        } else if side_tangent < 0.0 {
                            ui.label(
                                RichText::new("Shape: Positive trapezoid")
                                    .color(egui::Color32::from_rgb(0, 128, 255)),
                            );
                            ui.label("(top narrower than bottom)");
                        } else {
                            ui.label(
                                RichText::new("Shape: Rectangle")
                                    .color(egui::Color32::from_rgb(128, 128, 128)),
                            );
                        }
                    }

                    if let Some(resistive_etch) = layer.resistive_only_etch {
                        ui.label(format!("Resistive etch: {resistive_etch:.6} μm"));
                    }

                    if let Some(capacitive_etch) = layer.capacitive_only_etch {
                        ui.label(format!("Capacitive etch: {capacitive_etch:.6} μm"));
                    }

                    if layer.physical_props.width_min.is_none()
                        && layer.physical_props.spacing_min.is_none()
                        && layer.physical_props.side_tangent.is_none()
                        && layer.resistive_only_etch.is_none()
                        && layer.capacitive_only_etch.is_none()
                    {
                        ui.label("No physical properties available");
                    }
                });
        }

        if self.show_lookup_tables {
            self.show_lookup_tables_info(ui, layer);
        }
    }

    fn show_lookup_tables_info(&self, ui: &mut egui::Ui, layer: &crate::data::ConductorLayer) {
        CollapsingHeader::new("Lookup Tables")
            .default_open(false)
            .show(ui, |ui| {
                if let Some(ref rho_table) = layer.rho_vs_width_spacing {
                    CollapsingHeader::new("Resistivity vs Width/Spacing")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.label(format!("Width points: {}", rho_table.widths.len()));
                            ui.label(format!("Spacing points: {}", rho_table.spacings.len()));
                            ui.label(format!(
                                "Value matrix: {}×{}",
                                rho_table.values.len(),
                                rho_table.values.first().map(|row| row.len()).unwrap_or(0)
                            ));

                            if !rho_table.widths.is_empty() {
                                ui.label(format!(
                                    "Width range: {:.6} - {:.6} μm",
                                    rho_table.widths.first().unwrap_or(&0.0),
                                    rho_table.widths.last().unwrap_or(&0.0)
                                ));
                            }

                            if !rho_table.spacings.is_empty() {
                                ui.label(format!(
                                    "Spacing range: {:.6} - {:.6} μm",
                                    rho_table.spacings.first().unwrap_or(&0.0),
                                    rho_table.spacings.last().unwrap_or(&0.0)
                                ));
                            }

                            // Show a sample of the data if available
                            if !rho_table.values.is_empty() && !rho_table.values[0].is_empty() {
                                ui.separator();
                                ui.label("Sample values (first few entries):");
                                let max_rows = 3.min(rho_table.values.len());
                                let max_cols = 3.min(rho_table.values[0].len());

                                for i in 0..max_rows {
                                    let mut row_text = String::new();
                                    for j in 0..max_cols {
                                        if j > 0 {
                                            row_text.push_str(", ");
                                        }
                                        row_text
                                            .push_str(&format!("{:.3e}", rho_table.values[i][j]));
                                    }
                                    if max_cols < rho_table.values[i].len() {
                                        row_text.push_str(", ...");
                                    }
                                    ui.label(format!("[{i}]: {row_text}"));
                                }
                                if max_rows < rho_table.values.len() {
                                    ui.label("...");
                                }
                            }
                        });
                }

                if let Some(ref etch_table) = layer.etch_vs_width_spacing {
                    CollapsingHeader::new("Etch vs Width/Spacing")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.label(format!("Width points: {}", etch_table.widths.len()));
                            ui.label(format!("Spacing points: {}", etch_table.spacings.len()));
                            ui.label(format!(
                                "Value matrix: {}×{}",
                                etch_table.values.len(),
                                etch_table.values.first().map(|row| row.len()).unwrap_or(0)
                            ));

                            if !etch_table.widths.is_empty() {
                                ui.label(format!(
                                    "Width range: {:.6} - {:.6} μm",
                                    etch_table.widths.first().unwrap_or(&0.0),
                                    etch_table.widths.last().unwrap_or(&0.0)
                                ));
                            }

                            if !etch_table.spacings.is_empty() {
                                ui.label(format!(
                                    "Spacing range: {:.6} - {:.6} μm",
                                    etch_table.spacings.first().unwrap_or(&0.0),
                                    etch_table.spacings.last().unwrap_or(&0.0)
                                ));
                            }
                        });
                }

                if let Some(ref thickness_table) = layer.thickness_vs_width_spacing {
                    CollapsingHeader::new("Thickness vs Width/Spacing")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.label(format!("Width points: {}", thickness_table.widths.len()));
                            ui.label(format!(
                                "Spacing points: {}",
                                thickness_table.spacings.len()
                            ));
                            ui.label(format!(
                                "Value matrix: {}×{}",
                                thickness_table.values.len(),
                                thickness_table
                                    .values
                                    .first()
                                    .map(|row| row.len())
                                    .unwrap_or(0)
                            ));

                            if !thickness_table.widths.is_empty() {
                                ui.label(format!(
                                    "Width range: {:.6} - {:.6} μm",
                                    thickness_table.widths.first().unwrap_or(&0.0),
                                    thickness_table.widths.last().unwrap_or(&0.0)
                                ));
                            }

                            if !thickness_table.spacings.is_empty() {
                                ui.label(format!(
                                    "Spacing range: {:.6} - {:.6} μm",
                                    thickness_table.spacings.first().unwrap_or(&0.0),
                                    thickness_table.spacings.last().unwrap_or(&0.0)
                                ));
                            }
                        });
                }

                if layer.rho_vs_width_spacing.is_none()
                    && layer.etch_vs_width_spacing.is_none()
                    && layer.thickness_vs_width_spacing.is_none()
                {
                    ui.label("No lookup tables available");
                }
            });
    }

    fn show_via_details(&self, ui: &mut egui::Ui, via: &crate::data::ViaConnection) {
        CollapsingHeader::new("Via Properties")
            .default_open(true)
            .show(ui, |ui| {
                ui.label(format!("Name: {}", via.name));
                ui.label("Type: Via Connection");
                ui.label(format!("From layer: {}", via.from_layer));
                ui.label(format!("To layer: {}", via.to_layer));
                ui.label(format!("Area: {:.6} μm²", via.area));
                ui.label(format!("Width: {:.6} μm", via.get_via_width()));
                ui.label(format!("Height: {:.6} μm", via.height));
                ui.label(format!("Z position: {:.6} μm", via.z_position));
                ui.label(format!(
                    "Resistance per via: {:.3} Ω",
                    via.resistance_per_via
                ));
            });

        CollapsingHeader::new("Via Classification")
            .default_open(true)
            .show(ui, |ui| {
                ui.label(format!("Via type: {:?}", via.get_via_type()));

                if via.is_contact_via() {
                    ui.label(
                        RichText::new("Contact via").color(egui::Color32::from_rgb(255, 165, 0)),
                    );
                    ui.label("(connects to substrate/diffusion)");
                } else if via.is_metal_via() {
                    ui.label(
                        RichText::new("Metal via").color(egui::Color32::from_rgb(192, 192, 192)),
                    );
                    ui.label("(connects metal layers)");
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

impl Default for LayerDetailsPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_details_panel_creation() {
        let panel = LayerDetailsPanel::new();
        assert!(panel.is_open);
        assert!(panel.selected_layer.is_none());
        assert!(panel.show_electrical_props);
        assert!(panel.show_physical_props);
        assert!(!panel.show_lookup_tables);
    }

    #[test]
    fn test_layer_selection() {
        let mut panel = LayerDetailsPanel::new();

        panel.set_selected_layer(Some("metal1".to_string()));
        assert_eq!(panel.get_selected_layer(), Some(&"metal1".to_string()));

        panel.set_selected_layer(None);
        assert_eq!(panel.get_selected_layer(), None);
    }

    #[test]
    fn test_panel_visibility() {
        let mut panel = LayerDetailsPanel::new();
        assert!(panel.is_open);

        panel.toggle_visibility();
        assert!(!panel.is_open);

        panel.toggle_visibility();
        assert!(panel.is_open);
    }

    #[test]
    fn test_property_display_flags() {
        let mut panel = LayerDetailsPanel::new();

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
