// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use crate::data::ProcessStack;
use crate::renderer::{StackRenderer, ViewTransform};
use egui::{CentralPanel, Color32, Context, CursorIcon, Frame, Pos2, Sense, Vec2};

pub struct StackViewer {
    renderer: StackRenderer,
    transform: ViewTransform,
    is_panning: bool,
    last_mouse_pos: Option<Pos2>,
    zoom_sensitivity: f32,
    pan_sensitivity: f32,
}

impl StackViewer {
    pub fn new() -> Self {
        Self {
            renderer: StackRenderer::new(),
            transform: ViewTransform::new(Vec2::new(800.0, 600.0)),
            is_panning: false,
            last_mouse_pos: None,
            zoom_sensitivity: 1.1,
            pan_sensitivity: 1.0,
        }
    }

    pub fn show(&mut self, ctx: &Context, stack: Option<&ProcessStack>) -> Option<String> {
        let mut selected_layer = None;

        CentralPanel::default()
            .frame(Frame::canvas(&ctx.style()))
            .show(ctx, |ui| {
                // Update viewport size
                let viewport_rect = ui.available_rect_before_wrap();
                self.transform.viewport_size = viewport_rect.size();

                // Handle input
                let response = ui.allocate_rect(viewport_rect, Sense::click_and_drag());

                // Handle mouse interactions
                self.handle_mouse_input(ui, &response);

                // Handle keyboard shortcuts
                self.handle_keyboard_input(ui);

                if let Some(stack) = stack {
                    // Render the stack
                    let shapes = self
                        .renderer
                        .render_stack(stack, &self.transform, viewport_rect);

                    // Add shapes to the painter (layer names are now included in shapes for proper z-order)
                    let painter = ui.painter_at(viewport_rect);
                    for shape in shapes {
                        painter.add(shape);
                    }

                    // Handle layer selection via mouse click
                    if response.clicked() {
                        if let Some(mouse_pos) = response.interact_pointer_pos() {
                            selected_layer = self.renderer.hit_test(
                                stack,
                                &self.transform,
                                viewport_rect,
                                mouse_pos,
                            );

                            if let Some(ref layer_name) = selected_layer {
                                self.renderer.set_selected_layer(Some(layer_name.clone()));
                            }
                        }
                    }

                    // Show status information
                    self.show_status_overlay(ui, stack, viewport_rect);
                } else {
                    // Show message when no file is loaded
                    ui.centered_and_justified(|ui| {
                        ui.label("No ITF file loaded. Use File menu to open an ITF file.");
                    });
                }
            });

        selected_layer
    }

    fn handle_mouse_input(&mut self, ui: &mut egui::Ui, response: &egui::Response) {
        // Handle scrolling for zoom
        if response.hovered() {
            let scroll_delta = ui.input(|i| i.raw_scroll_delta);
            if scroll_delta.y != 0.0 {
                let zoom_factor = if scroll_delta.y > 0.0 {
                    self.zoom_sensitivity
                } else {
                    1.0 / self.zoom_sensitivity
                };

                let zoom_center = response
                    .interact_pointer_pos()
                    .unwrap_or_else(|| response.rect.center());

                self.transform.zoom(zoom_factor, zoom_center);
            }
        }

        // Handle panning
        if response.dragged() {
            if let Some(current_pos) = response.interact_pointer_pos() {
                if let Some(last_pos) = self.last_mouse_pos {
                    let delta = (current_pos - last_pos) * self.pan_sensitivity;
                    self.transform.pan(delta);
                }
                self.last_mouse_pos = Some(current_pos);
                self.is_panning = true;
            }
        } else {
            self.is_panning = false;
            self.last_mouse_pos = None;
        }

        // Set cursor icon once at the end based on final state - prevents race conditions
        let cursor_icon = if self.is_panning || response.dragged() {
            CursorIcon::Grabbing
        } else if response.hovered() {
            CursorIcon::Grab
        } else {
            // Only set to default if we're not in any interactive state
            if !self.is_panning {
                CursorIcon::Default
            } else {
                CursorIcon::Grab // Fallback to grab if we're still in an interactive state
            }
        };

        ui.output_mut(|output| output.cursor_icon = cursor_icon);
    }

    fn handle_keyboard_input(&mut self, ui: &mut egui::Ui) {
        let input = ui.input(|i| i.clone());

        // Zoom controls
        if input.key_pressed(egui::Key::Plus) || input.key_pressed(egui::Key::Equals) {
            let center = self.transform.viewport_size * 0.5;
            self.transform
                .zoom(self.zoom_sensitivity, Pos2::new(center.x, center.y));
        }

        if input.key_pressed(egui::Key::Minus) {
            let center = self.transform.viewport_size * 0.5;
            self.transform
                .zoom(1.0 / self.zoom_sensitivity, Pos2::new(center.x, center.y));
        }

        // Pan controls
        let pan_step = 20.0 / self.transform.scale;
        if input.key_pressed(egui::Key::ArrowLeft) {
            self.transform.pan(Vec2::new(pan_step, 0.0));
        }
        if input.key_pressed(egui::Key::ArrowRight) {
            self.transform.pan(Vec2::new(-pan_step, 0.0));
        }
        if input.key_pressed(egui::Key::ArrowUp) {
            self.transform.pan(Vec2::new(0.0, pan_step));
        }
        if input.key_pressed(egui::Key::ArrowDown) {
            self.transform.pan(Vec2::new(0.0, -pan_step));
        }

        // Reset view
        if input.key_pressed(egui::Key::R) && input.modifiers.ctrl {
            self.reset_view();
        }
    }

    fn show_status_overlay(
        &self,
        ui: &mut egui::Ui,
        stack: &ProcessStack,
        viewport_rect: egui::Rect,
    ) {
        let overlay_rect = egui::Rect::from_min_size(
            viewport_rect.min + Vec2::new(10.0, 10.0),
            Vec2::new(200.0, 100.0),
        );

        ui.scope_builder(egui::UiBuilder::new().max_rect(overlay_rect), |ui| {
            egui::Frame::popup(ui.style())
                .fill(Color32::from_black_alpha(200))
                .show(ui, |ui| {
                    ui.label(format!("Scale: {:.2}x", self.transform.scale));
                    ui.label(format!("Layers: {}", stack.get_layer_count()));
                    ui.label(format!("Height: {:.3} um", stack.get_total_height()));

                    if let Some(selected) = self.renderer.get_selected_layer() {
                        ui.label(format!("Selected: {selected}"));
                    }

                    ui.separator();
                    ui.label("Controls:");
                    ui.label("• Mouse wheel: Zoom");
                    ui.label("• Drag: Pan");
                    ui.label("• Click: Select layer");
                    ui.label("• Ctrl+R: Reset view");
                });
        });
    }

    pub fn auto_fit(&mut self, stack: &ProcessStack) {
        self.renderer.auto_fit(stack, &mut self.transform);
    }

    pub fn reset_view(&mut self) {
        self.transform = ViewTransform::new(self.transform.viewport_size);
    }

    pub fn set_selected_layer(&mut self, layer_name: Option<String>) {
        self.renderer.set_selected_layer(layer_name);
    }

    pub fn get_selected_layer(&self) -> Option<&String> {
        self.renderer.get_selected_layer()
    }

    pub fn set_show_dimensions(&mut self, show: bool) {
        self.renderer.set_show_dimensions(show);
    }

    pub fn set_show_layer_names(&mut self, show: bool) {
        self.renderer.set_show_layer_names(show);
    }

    pub fn set_show_schematic_mode(&mut self, show: bool) {
        self.renderer.set_show_schematic_mode(show);
    }

    pub fn set_layer_width(&mut self, width: f32) {
        self.renderer.set_layer_width(width);
    }

    pub fn get_zoom(&self) -> f32 {
        self.transform.scale
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        let center = self.transform.viewport_size * 0.5;
        // Remove upper limit, only keep minimum zoom
        let target_zoom = zoom.max(0.01);
        let zoom_factor = target_zoom / self.transform.scale;
        self.transform
            .zoom(zoom_factor, Pos2::new(center.x, center.y));
    }

    pub fn zoom_in(&mut self) {
        let center = self.transform.viewport_size * 0.5;
        self.transform
            .zoom(self.zoom_sensitivity, Pos2::new(center.x, center.y));
    }

    pub fn zoom_out(&mut self) {
        let center = self.transform.viewport_size * 0.5;
        self.transform
            .zoom(1.0 / self.zoom_sensitivity, Pos2::new(center.x, center.y));
    }

    pub fn pan(&mut self, delta: Vec2) {
        self.transform.pan(delta);
    }

    pub fn center_on_layer(&mut self, stack: &ProcessStack, layer_name: &str) {
        if let Some(layer) = stack.get_layer(layer_name) {
            let layer_center_z = (layer.get_bottom_z() + layer.get_top_z()) * 0.5;
            let world_center = Pos2::new(0.0, -(layer_center_z as f32));
            let screen_center = self.transform.viewport_size * 0.5;

            // Calculate offset to center the layer
            let current_screen_pos = self.transform.world_to_screen(world_center);
            let delta = Vec2::new(
                screen_center.x - current_screen_pos.x,
                screen_center.y - current_screen_pos.y,
            );

            self.transform.pan(delta);
        }
    }

    pub fn get_visible_bounds(&self) -> egui::Rect {
        self.transform.get_visible_world_bounds()
    }
}

impl Default for StackViewer {
    fn default() -> Self {
        Self::new()
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
        stack.add_layer(Layer::Dielectric(DielectricLayer::new(
            "oxide2".to_string(),
            1.5,
            4.2,
        )));

        stack
    }

    #[test]
    fn test_stack_viewer_creation() {
        let viewer = StackViewer::new();
        assert!(!viewer.is_panning);
        assert!(viewer.last_mouse_pos.is_none());
        assert!(viewer.zoom_sensitivity > 1.0);
        assert_eq!(viewer.pan_sensitivity, 1.0);
    }

    #[test]
    fn test_zoom_controls() {
        let mut viewer = StackViewer::new();
        let initial_zoom = viewer.get_zoom();

        viewer.zoom_in();
        assert!(viewer.get_zoom() > initial_zoom);

        viewer.zoom_out();
        assert!((viewer.get_zoom() - initial_zoom).abs() < 0.01);

        viewer.set_zoom(2.0);
        assert!((viewer.get_zoom() - 2.0).abs() < 0.01);

        // Test zoom bounds (no upper limit now)
        viewer.set_zoom(20.0); // Should work now
        assert!((viewer.get_zoom() - 20.0).abs() < 0.01);

        viewer.set_zoom(0.005); // Below min
        assert!(viewer.get_zoom() >= 0.01);
    }

    #[test]
    fn test_view_reset() {
        let mut viewer = StackViewer::new();
        let initial_scale = viewer.transform.scale;
        let initial_offset = viewer.transform.offset;

        // Modify view
        viewer.zoom_in();
        viewer.pan(Vec2::new(100.0, 50.0));

        assert_ne!(viewer.transform.scale, initial_scale);
        assert_ne!(viewer.transform.offset, initial_offset);

        // Reset view
        viewer.reset_view();
        assert_eq!(viewer.transform.scale, initial_scale);
        assert_eq!(viewer.transform.offset, initial_offset);
    }

    #[test]
    fn test_layer_selection() {
        let mut viewer = StackViewer::new();

        viewer.set_selected_layer(Some("metal1".to_string()));
        assert_eq!(viewer.get_selected_layer(), Some(&"metal1".to_string()));

        viewer.set_selected_layer(None);
        assert_eq!(viewer.get_selected_layer(), None);
    }

    #[test]
    fn test_display_options() {
        let mut viewer = StackViewer::new();

        viewer.set_show_dimensions(false);
        viewer.set_show_layer_names(false);
        viewer.set_layer_width(300.0);

        // These methods should not panic and should update internal state
        // The actual rendering is tested in the renderer module
    }

    #[test]
    fn test_auto_fit() {
        let mut viewer = StackViewer::new();
        let stack = create_test_stack();

        let initial_scale = viewer.transform.scale;
        let initial_offset = viewer.transform.offset;

        viewer.auto_fit(&stack);

        // Auto fit should change the view to encompass the stack
        // Exact values depend on the implementation, but scale and offset should change
        assert_ne!(viewer.transform.scale, initial_scale);
        assert_ne!(viewer.transform.offset, initial_offset);
    }

    #[test]
    fn test_center_on_layer() {
        let mut viewer = StackViewer::new();
        let stack = create_test_stack();

        let initial_offset = viewer.transform.offset;

        viewer.center_on_layer(&stack, "metal1");

        // Centering should change the offset
        assert_ne!(viewer.transform.offset, initial_offset);
    }

    #[test]
    fn test_pan_operations() {
        let mut viewer = StackViewer::new();
        let initial_offset = viewer.transform.offset;

        let delta = Vec2::new(50.0, -25.0);
        viewer.pan(delta);

        assert_ne!(viewer.transform.offset, initial_offset);
        assert_eq!(viewer.transform.offset, initial_offset + delta);
    }

    #[test]
    fn test_visible_bounds() {
        let viewer = StackViewer::new();
        let bounds = viewer.get_visible_bounds();

        // Should return a valid rectangle
        assert!(bounds.width() > 0.0);
        assert!(bounds.height() > 0.0);
    }
}
