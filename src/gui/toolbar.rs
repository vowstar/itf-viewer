// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use egui::{Context, Slider, TopBottomPanel};

pub struct Toolbar {
    pub show_dimensions: bool,
    pub show_layer_names: bool,
    pub layer_width: f32,
    pub zoom_level: f32,
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            show_dimensions: true,
            show_layer_names: true,
            layer_width: 200.0,
            zoom_level: 1.0,
        }
    }

    pub fn show(&mut self, ctx: &Context) -> ToolbarAction {
        let mut action = ToolbarAction::None;

        TopBottomPanel::top("toolbar")
            .resizable(false)
            .min_height(32.0)
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    // File operations
                    ui.menu_button("File", |ui| {
                        if ui.button("Open ITF File").clicked() {
                            action = ToolbarAction::OpenFile;
                            ui.close_menu();
                        }

                        ui.separator();

                        if ui.button("Exit").clicked() {
                            action = ToolbarAction::Exit;
                            ui.close_menu();
                        }
                    });

                    ui.separator();

                    // View controls
                    ui.menu_button("View", |ui| {
                        if ui
                            .checkbox(&mut self.show_dimensions, "Show Dimensions")
                            .clicked()
                        {
                            action = ToolbarAction::ToggleDimensions(self.show_dimensions);
                        }

                        if ui
                            .checkbox(&mut self.show_layer_names, "Show Layer Names")
                            .clicked()
                        {
                            action = ToolbarAction::ToggleLayerNames(self.show_layer_names);
                        }

                        ui.separator();

                        if ui.button("Auto Fit").clicked() {
                            action = ToolbarAction::AutoFit;
                            ui.close_menu();
                        }

                        if ui.button("Reset View").clicked() {
                            action = ToolbarAction::ResetView;
                            ui.close_menu();
                        }

                        ui.separator();

                        if ui.button("Zoom In").clicked() {
                            action = ToolbarAction::ZoomIn;
                            ui.close_menu();
                        }

                        if ui.button("Zoom Out").clicked() {
                            action = ToolbarAction::ZoomOut;
                            ui.close_menu();
                        }
                    });

                    ui.separator();

                    if ui.button("Zoom+").on_hover_text("Zoom in").clicked() {
                        action = ToolbarAction::ZoomIn;
                    }

                    if ui.button("Zoom-").on_hover_text("Zoom out").clicked() {
                        action = ToolbarAction::ZoomOut;
                    }

                    if ui.button("Fit").on_hover_text("Auto fit").clicked() {
                        action = ToolbarAction::AutoFit;
                    }

                    if ui.button("Reset").on_hover_text("Reset view").clicked() {
                        action = ToolbarAction::ResetView;
                    }

                    ui.separator();

                    // Zoom level display and control
                    ui.label("Zoom:");
                    let zoom_response = ui.add(
                        Slider::new(&mut self.zoom_level, 0.01..=1000.0)
                            .step_by(0.01)
                            .suffix("x")
                            .logarithmic(true),
                    );

                    if zoom_response.changed() {
                        action = ToolbarAction::SetZoom(self.zoom_level);
                    }

                    ui.separator();

                    // Layer width control
                    ui.label("Width:");
                    let width_response = ui.add(
                        Slider::new(&mut self.layer_width, 50.0..=500.0)
                            .step_by(10.0)
                            .suffix(" px"),
                    );

                    if width_response.changed() {
                        action = ToolbarAction::SetLayerWidth(self.layer_width);
                    }

                    ui.separator();

                    // Display toggles
                    let dim_response = ui.checkbox(&mut self.show_dimensions, "Dimensions");
                    if dim_response.clicked() {
                        action = ToolbarAction::ToggleDimensions(self.show_dimensions);
                    }

                    let names_response = ui.checkbox(&mut self.show_layer_names, "Names");
                    if names_response.clicked() {
                        action = ToolbarAction::ToggleLayerNames(self.show_layer_names);
                    }
                });
            });

        action
    }

    pub fn update_zoom(&mut self, zoom: f32) {
        self.zoom_level = zoom;
    }

    pub fn set_show_dimensions(&mut self, show: bool) {
        self.show_dimensions = show;
    }

    pub fn set_show_layer_names(&mut self, show: bool) {
        self.show_layer_names = show;
    }

    pub fn set_layer_width(&mut self, width: f32) {
        self.layer_width = width;
    }
}

impl Default for Toolbar {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToolbarAction {
    None,
    OpenFile,
    Exit,
    AutoFit,
    ResetView,
    ZoomIn,
    ZoomOut,
    SetZoom(f32),
    SetLayerWidth(f32),
    ToggleDimensions(bool),
    ToggleLayerNames(bool),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toolbar_creation() {
        let toolbar = Toolbar::new();
        assert!(toolbar.show_dimensions);
        assert!(toolbar.show_layer_names);
        assert_eq!(toolbar.layer_width, 200.0);
        assert_eq!(toolbar.zoom_level, 1.0);
    }

    #[test]
    fn test_toolbar_updates() {
        let mut toolbar = Toolbar::new();

        toolbar.update_zoom(2.5);
        assert_eq!(toolbar.zoom_level, 2.5);

        toolbar.set_show_dimensions(false);
        assert!(!toolbar.show_dimensions);

        toolbar.set_show_layer_names(false);
        assert!(!toolbar.show_layer_names);

        toolbar.set_layer_width(350.0);
        assert_eq!(toolbar.layer_width, 350.0);
    }

    #[test]
    fn test_toolbar_action_enum() {
        // Test that the enum variants are properly defined
        let action1 = ToolbarAction::None;
        let action2 = ToolbarAction::OpenFile;
        let action3 = ToolbarAction::SetZoom(1.5);
        let action4 = ToolbarAction::ToggleDimensions(true);

        assert_ne!(action1, action2);
        assert_ne!(action2, action3);
        assert_ne!(action3, action4);

        // Test specific values
        match action3 {
            ToolbarAction::SetZoom(zoom) => assert_eq!(zoom, 1.5),
            _ => panic!("Expected SetZoom action"),
        }

        match action4 {
            ToolbarAction::ToggleDimensions(show) => assert!(show),
            _ => panic!("Expected ToggleDimensions action"),
        }
    }

    #[test]
    fn test_default_implementation() {
        let toolbar1 = Toolbar::new();
        let toolbar2 = Toolbar::default();

        assert_eq!(toolbar1.show_dimensions, toolbar2.show_dimensions);
        assert_eq!(toolbar1.show_layer_names, toolbar2.show_layer_names);
        assert_eq!(toolbar1.layer_width, toolbar2.layer_width);
        assert_eq!(toolbar1.zoom_level, toolbar2.zoom_level);
    }

    #[test]
    fn test_clone_and_debug_for_actions() {
        let action = ToolbarAction::SetLayerWidth(250.0);
        let cloned_action = action.clone();

        assert_eq!(action, cloned_action);

        // Test Debug formatting (should not panic)
        let debug_str = format!("{action:?}");
        assert!(debug_str.contains("SetLayerWidth"));
        assert!(debug_str.contains("250"));
    }

    #[test]
    fn test_action_matching() {
        let actions = vec![
            ToolbarAction::None,
            ToolbarAction::OpenFile,
            ToolbarAction::Exit,
            ToolbarAction::AutoFit,
            ToolbarAction::ResetView,
            ToolbarAction::ZoomIn,
            ToolbarAction::ZoomOut,
            ToolbarAction::SetZoom(2.0),
            ToolbarAction::SetLayerWidth(300.0),
            ToolbarAction::ToggleDimensions(false),
            ToolbarAction::ToggleLayerNames(true),
        ];

        for action in actions {
            match action {
                ToolbarAction::None => {}
                ToolbarAction::OpenFile => {}
                ToolbarAction::Exit => {}
                ToolbarAction::AutoFit => {}
                ToolbarAction::ResetView => {}
                ToolbarAction::ZoomIn => {}
                ToolbarAction::ZoomOut => {}
                ToolbarAction::SetZoom(_) => {}
                ToolbarAction::SetLayerWidth(_) => {}
                ToolbarAction::ToggleDimensions(_) => {}
                ToolbarAction::ToggleLayerNames(_) => {}
            }
        }
    }
}
