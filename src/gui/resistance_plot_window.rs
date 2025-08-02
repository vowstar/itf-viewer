// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use crate::data::{ConductorLayer, Layer, ProcessStack};
use egui::{CollapsingHeader, ComboBox, Context, DragValue, Grid, Window};
use egui_plot::{Line, Plot, PlotPoints};

#[derive(Clone, Debug)]
pub struct ResistanceCurve {
    pub name: String,
    pub data_points: Vec<(f64, f64)>, // (temperature, resistance) pairs
    pub color: egui::Color32,
}

pub struct ResistancePlotWindow {
    open: bool,
    // Input parameters
    selected_conductor: Option<String>,
    width: f64,             // Line width in micrometers
    length: f64,            // Line length in micrometers
    temperature_start: f64, // Start temperature for plot
    temperature_end: f64,   // End temperature for plot
    reference_temp: f64,    // Reference temperature (usually 25°C)

    // Results
    calculated_resistance: Option<f64>,
    calculated_sheet_resistance: Option<f64>,
    curves: Vec<ResistanceCurve>,
    curves_generated: bool,
    error_message: Option<String>,

    // Display settings
    plot_title: String,
    x_axis_label: String,
    y_axis_label: String,
}

impl ResistancePlotWindow {
    pub fn new() -> Self {
        Self {
            open: false,
            // Input parameters
            selected_conductor: None,
            width: 0.1,               // Default 0.1 μm
            length: 100.0,            // Default 100 μm
            temperature_start: -40.0, // -40°C
            temperature_end: 150.0,   // 150°C
            reference_temp: 25.0,     // 25°C

            // Results
            calculated_resistance: None,
            calculated_sheet_resistance: None,
            curves: Vec::new(),
            curves_generated: false,
            error_message: None,

            // Display settings
            plot_title: "Resistance vs Temperature".to_string(),
            x_axis_label: "Temperature (°C)".to_string(),
            y_axis_label: "Resistance (Ω)".to_string(),
        }
    }

    pub fn set_selected_conductor(&mut self, conductor_name: Option<String>) {
        self.selected_conductor = conductor_name;
        // Clear calculated values when layer changes
        self.calculated_resistance = None;
        self.calculated_sheet_resistance = None;
        self.curves_generated = false;
        self.curves.clear();
        self.error_message = None;
    }

    pub fn set_open(&mut self, open: bool) {
        self.open = open;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn show(&mut self, ctx: &Context, stack: Option<&ProcessStack>) {
        if !self.open {
            return;
        }

        let mut open = self.open;
        Window::new("Resistance Calculator")
            .open(&mut open)
            .default_size([900.0, 700.0])
            .resizable(true)
            .scroll([false, true])
            .show(ctx, |ui| {
                self.show_content(ui, stack);
            });
        self.open = open;
    }

    fn show_content(&mut self, ui: &mut egui::Ui, stack: Option<&ProcessStack>) {
        ui.heading("Resistance Calculator");

        // Input controls
        self.show_input_controls(ui, stack);

        ui.separator();

        // Results display
        self.show_results(ui);

        ui.separator();

        // Plot display
        if self.curves_generated && !self.curves.is_empty() {
            self.show_temperature_plot(ui);
        } else {
            ui.label("Calculate resistance first to generate temperature curves");
        }

        // Error message display
        if let Some(ref error) = self.error_message {
            ui.separator();
            ui.colored_label(egui::Color32::RED, format!("Error: {error}"));
        }
    }

    fn show_input_controls(&mut self, ui: &mut egui::Ui, stack: Option<&ProcessStack>) {
        CollapsingHeader::new("Input Parameters")
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("resistance_inputs")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .show(ui, |ui| {
                        // Conductor layer selection
                        ui.label("Conductor Layer:");
                        let current_selection = self
                            .selected_conductor
                            .as_deref()
                            .unwrap_or("Select layer...");

                        ComboBox::from_id_salt("conductor_selection")
                            .selected_text(current_selection)
                            .show_ui(ui, |ui| {
                                if let Some(stack) = stack {
                                    for layer in &stack.layers {
                                        if let Layer::Conductor(conductor) = layer {
                                            if ui
                                                .selectable_label(
                                                    self.selected_conductor.as_ref()
                                                        == Some(&conductor.name),
                                                    &conductor.name,
                                                )
                                                .clicked()
                                            {
                                                self.selected_conductor =
                                                    Some(conductor.name.clone());
                                                // Clear calculated values when layer changes
                                                self.calculated_resistance = None;
                                                self.calculated_sheet_resistance = None;
                                                self.curves_generated = false;
                                                self.curves.clear();
                                                self.error_message = None;
                                            }
                                        }
                                    }
                                }
                            });
                        ui.end_row();

                        // Width input
                        ui.label("Width (μm):");
                        let width_response = ui.add(
                            DragValue::new(&mut self.width)
                                .range(0.001..=1000.0)
                                .speed(0.01)
                                .suffix(" μm"),
                        );
                        if width_response.changed() {
                            self.clear_results();
                        }
                        ui.end_row();

                        // Length input
                        ui.label("Length (μm):");
                        let length_response = ui.add(
                            DragValue::new(&mut self.length)
                                .range(0.001..=10000.0)
                                .speed(0.1)
                                .suffix(" μm"),
                        );
                        if length_response.changed() {
                            self.clear_results();
                        }
                        ui.end_row();

                        // Temperature range for plot
                        ui.label("Temperature Range:");
                        ui.horizontal(|ui| {
                            ui.add(
                                DragValue::new(&mut self.temperature_start)
                                    .range(-100.0..=200.0)
                                    .speed(1.0)
                                    .suffix("°C"),
                            );
                            ui.label("to");
                            ui.add(
                                DragValue::new(&mut self.temperature_end)
                                    .range(-100.0..=200.0)
                                    .speed(1.0)
                                    .suffix("°C"),
                            );
                        });
                        ui.end_row();

                        // Calculate button
                        ui.label("");
                        if ui.button("Calculate & Plot").clicked() {
                            if let Some(stack) = stack {
                                self.calculate_resistance(stack);
                                self.generate_temperature_curves(stack);
                            }
                        }
                        ui.end_row();
                    });
            });
    }

    fn show_results(&mut self, ui: &mut egui::Ui) {
        CollapsingHeader::new("Results")
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("resistance_results")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .show(ui, |ui| {
                        // Resistance at reference temperature
                        ui.label("Resistance (25°C):");
                        if let Some(resistance) = self.calculated_resistance {
                            ui.label(format!("{resistance:.6} Ω"));
                        } else {
                            ui.label("Not calculated");
                        }
                        ui.end_row();

                        // Sheet resistance
                        ui.label("Sheet Resistance:");
                        if let Some(sheet_resistance) = self.calculated_sheet_resistance {
                            ui.label(format!("{sheet_resistance:.6} ohm/sq"));
                        } else {
                            ui.label("Not calculated");
                        }
                        ui.end_row();
                    });
            });
    }

    fn show_temperature_plot(&mut self, ui: &mut egui::Ui) {
        CollapsingHeader::new("Temperature vs Resistance Plot")
            .default_open(true)
            .show(ui, |ui| {
                // Update plot title with current parameters
                if let Some(ref conductor_name) = self.selected_conductor {
                    self.plot_title = format!(
                        "Resistance vs Temperature - {} (W={:.3}μm, L={:.1}μm)",
                        conductor_name, self.width, self.length
                    );
                }

                // Create the plot
                Plot::new("resistance_temperature_plot")
                    .view_aspect(2.0)
                    .legend(egui_plot::Legend::default())
                    .x_axis_label(&self.x_axis_label)
                    .y_axis_label(&self.y_axis_label)
                    .show(ui, |plot_ui| {
                        for curve in &self.curves {
                            if !curve.data_points.is_empty() {
                                let points: PlotPoints = curve
                                    .data_points
                                    .iter()
                                    .map(|(temp, resistance)| [*temp, *resistance])
                                    .collect();

                                let line = Line::new(&curve.name, points).color(curve.color);

                                plot_ui.line(line);
                            }
                        }
                    });

                ui.separator();

                // Show curve statistics
                self.show_curve_statistics(ui);
            });
    }

    fn show_curve_statistics(&self, ui: &mut egui::Ui) {
        if self.curves.is_empty() {
            return;
        }

        ui.collapsing("Curve Statistics", |ui| {
            for curve in &self.curves {
                if !curve.data_points.is_empty() {
                    ui.label(format!("Curve: {}", curve.name));

                    let resistances: Vec<f64> = curve.data_points.iter().map(|(_, r)| *r).collect();
                    let min_resistance = resistances.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                    let max_resistance =
                        resistances.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                    let avg_resistance = resistances.iter().sum::<f64>() / resistances.len() as f64;

                    ui.indent("curve_stats", |ui| {
                        ui.label(format!("  Min Resistance: {min_resistance:.6} Ω"));
                        ui.label(format!("  Max Resistance: {max_resistance:.6} Ω"));
                        ui.label(format!("  Avg Resistance: {avg_resistance:.6} Ω"));
                        ui.label(format!(
                            "  Variation: {:.2}%",
                            ((max_resistance - min_resistance) / avg_resistance * 100.0)
                        ));
                        ui.label(format!("  Data Points: {}", curve.data_points.len()));
                    });
                    ui.separator();
                }
            }
        });
    }

    fn clear_results(&mut self) {
        self.calculated_resistance = None;
        self.calculated_sheet_resistance = None;
        self.curves_generated = false;
        self.curves.clear();
        self.error_message = None;
    }

    fn calculate_resistance(&mut self, stack: &ProcessStack) {
        self.error_message = None;
        self.calculated_resistance = None;
        self.calculated_sheet_resistance = None;

        let conductor = match self.get_selected_conductor(stack) {
            Some(c) => c,
            None => {
                self.error_message = Some("No conductor layer selected".to_string());
                return;
            }
        };

        // Calculate resistance at reference temperature
        match conductor.calculate_resistance(
            self.width,
            self.length,
            self.reference_temp,
            self.reference_temp,
        ) {
            Some(resistance) => {
                self.calculated_resistance = Some(resistance);
                // Calculate sheet resistance: Rsq = R * (W/L)
                self.calculated_sheet_resistance = Some(resistance * self.width / self.length);
            }
            None => {
                self.error_message =
                    Some("Cannot calculate resistance - missing electrical properties".to_string());
            }
        }
    }

    fn generate_temperature_curves(&mut self, stack: &ProcessStack) {
        self.curves.clear();

        let conductor = match self.get_selected_conductor(stack) {
            Some(c) => c,
            None => return,
        };

        // Generate points from temperature_start to temperature_end
        let num_points = 100;
        let temp_step = (self.temperature_end - self.temperature_start) / (num_points as f64 - 1.0);

        let mut curve_data = Vec::new();
        for i in 0..num_points {
            let temperature = self.temperature_start + (i as f64) * temp_step;

            if let Some(resistance) = conductor.calculate_resistance(
                self.width,
                self.length,
                temperature,
                self.reference_temp,
            ) {
                curve_data.push((temperature, resistance));
            }
        }

        if !curve_data.is_empty() {
            let conductor_name = self
                .selected_conductor
                .as_ref()
                .unwrap_or(&"Unknown".to_string())
                .clone();
            let curve = ResistanceCurve {
                name: format!("{} (W={:.3}μm)", conductor_name, self.width),
                data_points: curve_data,
                color: egui::Color32::BLUE,
            };
            self.curves.push(curve);
            self.curves_generated = true;
        }
    }

    fn get_selected_conductor<'a>(&self, stack: &'a ProcessStack) -> Option<&'a ConductorLayer> {
        let conductor_name = self.selected_conductor.as_ref()?;

        for layer in &stack.layers {
            if let Layer::Conductor(conductor) = layer {
                if conductor.name == *conductor_name {
                    return Some(conductor);
                }
            }
        }
        None
    }
}

impl Default for ResistancePlotWindow {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resistance_plot_window_creation() {
        let window = ResistancePlotWindow::new();
        assert!(!window.open);
        assert!(window.curves.is_empty());
        assert!(window.selected_conductor.is_none());
        assert_eq!(window.width, 0.1);
        assert_eq!(window.length, 100.0);
        assert_eq!(window.temperature_start, -40.0);
        assert_eq!(window.temperature_end, 150.0);
        assert_eq!(window.reference_temp, 25.0);
        assert!(!window.curves_generated);
    }

    #[test]
    fn test_window_visibility_control() {
        let mut window = ResistancePlotWindow::new();

        assert!(!window.is_open());

        window.set_open(true);
        assert!(window.is_open());

        window.set_open(false);
        assert!(!window.is_open());
    }

    #[test]
    fn test_conductor_selection() {
        let mut window = ResistancePlotWindow::new();

        assert!(window.selected_conductor.is_none());

        window.set_selected_conductor(Some("metal1".to_string()));
        assert_eq!(window.selected_conductor, Some("metal1".to_string()));

        window.set_selected_conductor(None);
        assert!(window.selected_conductor.is_none());
    }

    #[test]
    fn test_clear_results() {
        let mut window = ResistancePlotWindow::new();

        // Set some test data
        window.calculated_resistance = Some(1.5);
        window.calculated_sheet_resistance = Some(0.5);
        window.curves_generated = true;
        window.curves.push(ResistanceCurve {
            name: "test".to_string(),
            data_points: vec![(25.0, 1.0)],
            color: egui::Color32::RED,
        });
        window.error_message = Some("test error".to_string());

        // Clear results
        window.clear_results();

        assert!(window.calculated_resistance.is_none());
        assert!(window.calculated_sheet_resistance.is_none());
        assert!(!window.curves_generated);
        assert!(window.curves.is_empty());
        assert!(window.error_message.is_none());
    }

    #[test]
    fn test_resistance_curve() {
        let curve = ResistanceCurve {
            name: "Test".to_string(),
            data_points: vec![(0.0, 1.0), (100.0, 1.5)],
            color: egui::Color32::BLUE,
        };

        assert_eq!(curve.name, "Test");
        assert_eq!(curve.data_points.len(), 2);
        assert_eq!(curve.data_points[0], (0.0, 1.0));
        assert_eq!(curve.data_points[1], (100.0, 1.5));
    }

    #[test]
    fn test_default_implementation() {
        let window1 = ResistancePlotWindow::new();
        let window2 = ResistancePlotWindow::default();

        assert_eq!(window1.open, window2.open);
        assert_eq!(window1.width, window2.width);
        assert_eq!(window1.length, window2.length);
        assert_eq!(window1.curves.len(), window2.curves.len());
        assert_eq!(window1.temperature_start, window2.temperature_start);
        assert_eq!(window1.temperature_end, window2.temperature_end);
    }

    #[test]
    fn test_parameter_defaults() {
        let window = ResistancePlotWindow::new();

        // Test default values match expected engineering defaults
        assert_eq!(window.width, 0.1); // 100nm - typical minimum feature size
        assert_eq!(window.length, 100.0); // 100μm - reasonable test length
        assert_eq!(window.temperature_start, -40.0); // Industrial temp range
        assert_eq!(window.temperature_end, 150.0); // Industrial temp range
        assert_eq!(window.reference_temp, 25.0); // Standard reference temperature
    }
}
