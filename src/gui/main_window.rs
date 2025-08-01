// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use crate::data::ProcessStack;
use crate::gui::{FileMenu, LayerDetailsPanel, LayerPanel, StackViewer, Toolbar, ToolbarAction};
use crate::parser::parse_itf_file;
use egui::Context;
use rfd::FileDialog;
use std::path::PathBuf;

pub struct MainWindow {
    file_menu: FileMenu,
    layer_panel: LayerPanel,
    layer_details_panel: LayerDetailsPanel,
    stack_viewer: StackViewer,
    toolbar: Toolbar,
    current_stack: Option<ProcessStack>,
    show_about: bool,
    error_message: Option<String>,
}

impl MainWindow {
    pub fn new() -> Self {
        Self {
            file_menu: FileMenu::new(),
            layer_panel: LayerPanel::new(),
            layer_details_panel: LayerDetailsPanel::new(),
            stack_viewer: StackViewer::new(),
            toolbar: Toolbar::new(),
            current_stack: None,
            show_about: false,
            error_message: None,
        }
    }

    pub fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Handle toolbar actions
        let toolbar_action = self.toolbar.show(ctx);
        self.handle_toolbar_action(toolbar_action);

        // Only show file menu if explicitly requested (for error display)
        if self.file_menu.is_open {
            self.file_menu.show(ctx);
        }

        // Check for newly loaded stack
        if self.file_menu.has_loaded_stack() {
            if let Some(stack) = self.file_menu.take_loaded_stack() {
                self.load_stack(stack);
            }
        }

        // Show layer panel and handle layer selection
        if let Some(selected_layer) = self.layer_panel.show(ctx, self.current_stack.as_ref()) {
            self.stack_viewer
                .set_selected_layer(Some(selected_layer.clone()));
            self.layer_panel
                .set_selected_layer(Some(selected_layer.clone()));
            self.layer_details_panel
                .set_selected_layer(Some(selected_layer));
        }

        // Show layer details panel on the right
        self.layer_details_panel
            .show(ctx, self.current_stack.as_ref());

        // Show main stack viewer and handle layer selection from viewer
        if let Some(selected_layer) = self.stack_viewer.show(ctx, self.current_stack.as_ref()) {
            self.layer_panel
                .set_selected_layer(Some(selected_layer.clone()));
            self.layer_details_panel
                .set_selected_layer(Some(selected_layer));
        }

        // Show about dialog if requested
        if self.show_about {
            self.show_about_dialog(ctx);
        }

        // Show error dialog if there's an error
        if self.error_message.is_some() {
            self.show_error_dialog_ui(ctx);
        }

        // Update toolbar state
        self.toolbar.update_zoom(self.stack_viewer.get_zoom());
    }

    fn handle_toolbar_action(&mut self, action: ToolbarAction) {
        match action {
            ToolbarAction::None => {}

            ToolbarAction::OpenFile => {
                self.open_file_dialog();
            }

            ToolbarAction::Exit => {
                std::process::exit(0);
            }

            ToolbarAction::AutoFit => {
                if let Some(ref stack) = self.current_stack {
                    self.stack_viewer.auto_fit(stack);
                }
            }

            ToolbarAction::ResetView => {
                self.stack_viewer.reset_view();
            }

            ToolbarAction::ZoomIn => {
                self.stack_viewer.zoom_in();
            }

            ToolbarAction::ZoomOut => {
                self.stack_viewer.zoom_out();
            }

            ToolbarAction::SetZoom(zoom) => {
                self.stack_viewer.set_zoom(zoom);
            }

            ToolbarAction::SetLayerWidth(width) => {
                self.stack_viewer.set_layer_width(width);
                self.toolbar.set_layer_width(width);
            }

            ToolbarAction::ToggleDimensions(show) => {
                self.stack_viewer.set_show_dimensions(show);
                self.toolbar.set_show_dimensions(show);
            }

            ToolbarAction::ToggleLayerNames(show) => {
                self.stack_viewer.set_show_layer_names(show);
                self.toolbar.set_show_layer_names(show);
            }

            ToolbarAction::ToggleSchematicMode(show) => {
                self.stack_viewer.set_show_schematic_mode(show);
                self.toolbar.set_show_schematic_mode(show);
            }
        }
    }

    fn load_stack(&mut self, stack: ProcessStack) {
        self.current_stack = Some(stack);

        // Auto-fit the new stack
        if let Some(ref stack) = self.current_stack {
            self.stack_viewer.auto_fit(stack);
        }

        // Clear any previous layer selection
        self.layer_panel.set_selected_layer(None);
        self.layer_details_panel.set_selected_layer(None);
        self.stack_viewer.set_selected_layer(None);

        // Close file menu
        self.file_menu.is_open = false;
    }

    fn show_about_dialog(&mut self, ctx: &Context) {
        egui::Window::new("About ITF Viewer")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("ITF Viewer");
                    ui.label("Version 0.1.0");
                    ui.separator();

                    ui.label(
                        "A cross-sectional viewer for ITF (Interconnect Technology Format) files.",
                    );
                    ui.label("Visualizes semiconductor process stacks with layers and vias.");

                    ui.separator();

                    ui.label("Features:");
                    ui.label("• Parse and display ITF process stacks");
                    ui.label("• Interactive pan, zoom, and layer selection");
                    ui.label("• Detailed layer property inspection");
                    ui.label("• Trapezoid visualization for metal layers");
                    ui.label("• Color-coded materials (copper/dielectric)");

                    ui.separator();

                    ui.label("Controls:");
                    ui.label("• Mouse wheel: Zoom in/out");
                    ui.label("• Drag: Pan view");
                    ui.label("• Click: Select layer");
                    ui.label("• Ctrl+R: Reset view");
                    ui.label("• Arrow keys: Pan view");
                    ui.label("• +/- keys: Zoom");

                    ui.separator();

                    if ui.button("Close").clicked() {
                        self.show_about = false;
                    }
                });
            });
    }

    pub fn get_current_stack(&self) -> Option<&ProcessStack> {
        self.current_stack.as_ref()
    }

    pub fn set_show_about(&mut self, show: bool) {
        self.show_about = show;
    }

    pub fn toggle_layer_panel(&mut self) {
        self.layer_panel.toggle_visibility();
    }

    pub fn get_selected_layer(&self) -> Option<&String> {
        self.layer_panel.get_selected_layer()
    }

    pub fn center_on_layer(&mut self, layer_name: &str) {
        if let Some(ref stack) = self.current_stack {
            self.stack_viewer.center_on_layer(stack, layer_name);
        }
    }

    pub fn select_layer(&mut self, layer_name: Option<String>) {
        self.layer_panel.set_selected_layer(layer_name.clone());
        self.stack_viewer.set_selected_layer(layer_name);
    }

    pub fn has_loaded_file(&self) -> bool {
        self.current_stack.is_some()
    }

    fn open_file_dialog(&mut self) {
        // Directly open native file dialog to select ITF files
        if let Some(path) = FileDialog::new()
            .add_filter("ITF Files", &["itf"])
            .add_filter("All Files", &["*"])
            .set_title("Select ITF File")
            .pick_file()
        {
            self.load_file_from_path(path);
        }
    }

    fn load_file_from_path(&mut self, path: PathBuf) {
        match std::fs::read_to_string(&path) {
            Ok(content) => match parse_itf_file(&content) {
                Ok(stack) => {
                    self.load_stack(stack);
                }
                Err(e) => {
                    self.show_error_dialog(&format!("Failed to parse ITF file: {e}"));
                }
            },
            Err(e) => {
                self.show_error_dialog(&format!("Failed to read file: {e}"));
            }
        }
    }

    fn show_error_dialog(&mut self, message: &str) {
        self.error_message = Some(message.to_string());
    }

    fn show_error_dialog_ui(&mut self, ctx: &Context) {
        if let Some(ref error_msg) = self.error_message.clone() {
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("Error")
                                .color(egui::Color32::RED)
                                .size(16.0),
                        );
                        ui.separator();

                        ui.label(error_msg);

                        ui.separator();

                        if ui.button("OK").clicked() {
                            self.error_message = None;
                        }
                    });
                });
        }
    }
}

impl Default for MainWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for MainWindow {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        self.update(ctx, frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{ConductorLayer, DielectricLayer, Layer, TechnologyInfo};

    fn create_test_stack() -> ProcessStack {
        let tech = TechnologyInfo::new("test_stack".to_string());
        let mut stack = ProcessStack::new(tech);

        stack.add_layer(Layer::Dielectric(DielectricLayer::new(
            "oxide1".to_string(),
            1.0,
            4.2,
        )));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new(
            "metal1".to_string(),
            0.5,
        ))));

        stack
    }

    #[test]
    fn test_main_window_creation() {
        let window = MainWindow::new();
        assert!(!window.file_menu.is_open);
        assert!(window.layer_panel.is_open);
        assert!(window.current_stack.is_none());
        assert!(!window.show_about);
    }

    #[test]
    fn test_stack_loading() {
        let mut window = MainWindow::new();
        let stack = create_test_stack();

        assert!(!window.has_loaded_file());
        assert!(window.get_current_stack().is_none());

        window.load_stack(stack);

        assert!(window.has_loaded_file());
        assert!(window.get_current_stack().is_some());
        assert!(!window.file_menu.is_open);
        assert!(window.layer_panel.get_selected_layer().is_none());
    }

    #[test]
    fn test_layer_selection() {
        let mut window = MainWindow::new();

        window.select_layer(Some("metal1".to_string()));
        assert_eq!(window.get_selected_layer(), Some(&"metal1".to_string()));

        window.select_layer(None);
        assert_eq!(window.get_selected_layer(), None);
    }

    #[test]
    fn test_about_dialog() {
        let mut window = MainWindow::new();
        assert!(!window.show_about);

        window.set_show_about(true);
        assert!(window.show_about);

        window.set_show_about(false);
        assert!(!window.show_about);
    }

    #[test]
    fn test_layer_panel_toggle() {
        let mut window = MainWindow::new();
        let initial_state = window.layer_panel.is_open;

        window.toggle_layer_panel();
        assert_ne!(window.layer_panel.is_open, initial_state);

        window.toggle_layer_panel();
        assert_eq!(window.layer_panel.is_open, initial_state);
    }

    #[test]
    fn test_toolbar_actions() {
        let mut window = MainWindow::new();

        // Skip OpenFile test to avoid opening system file browser during automated tests
        // Instead, test file loading directly using a test file
        window.load_file_from_path(PathBuf::from("tests/data/complex_test.itf"));
        assert!(window.has_loaded_file());

        // Test reset view action (should not panic)
        window.handle_toolbar_action(ToolbarAction::ResetView);

        // Test zoom actions
        window.handle_toolbar_action(ToolbarAction::ZoomIn);
        window.handle_toolbar_action(ToolbarAction::ZoomOut);
        window.handle_toolbar_action(ToolbarAction::SetZoom(2.0));

        // Test display toggles
        window.handle_toolbar_action(ToolbarAction::ToggleDimensions(false));
        assert!(!window.toolbar.show_dimensions);

        window.handle_toolbar_action(ToolbarAction::ToggleLayerNames(false));
        assert!(!window.toolbar.show_layer_names);

        // Test layer width setting
        window.handle_toolbar_action(ToolbarAction::SetLayerWidth(300.0));
        assert_eq!(window.toolbar.layer_width, 300.0);
    }

    #[test]
    fn test_auto_fit_without_stack() {
        let mut window = MainWindow::new();

        // Should not panic when no stack is loaded
        window.handle_toolbar_action(ToolbarAction::AutoFit);
    }

    #[test]
    fn test_center_on_layer() {
        let mut window = MainWindow::new();
        let stack = create_test_stack();
        window.load_stack(stack);

        // Should not panic when centering on existing layer
        window.center_on_layer("metal1");

        // Should not panic when centering on non-existing layer
        window.center_on_layer("nonexistent");
    }
}
