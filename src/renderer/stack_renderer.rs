// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use egui::{Pos2, Rect, Shape, Stroke, Color32};
use crate::data::{ProcessStack, Layer};
use crate::renderer::{colors::ColorScheme, geometry::*};

pub struct StackRenderer {
    color_scheme: ColorScheme,
    layer_width: f32,
    show_dimensions: bool,
    show_layer_names: bool,
    selected_layer: Option<String>,
}

impl StackRenderer {
    pub fn new() -> Self {
        Self {
            color_scheme: ColorScheme::new(),
            layer_width: 200.0,
            show_dimensions: true,
            show_layer_names: true,
            selected_layer: None,
        }
    }

    pub fn render_stack(
        &self,
        stack: &ProcessStack,
        transform: &ViewTransform,
        viewport_rect: Rect,
    ) -> Vec<Shape> {
        let mut shapes = Vec::new();
        
        // Calculate layer positions and create geometries
        let layer_geometries = self.create_layer_geometries(stack, transform, viewport_rect);
        let via_geometries = self.create_via_geometries(stack, transform, viewport_rect);
        
        // Render layers from bottom to top
        for geometry in &layer_geometries {
            shapes.extend(geometry.to_egui_shapes());
        }
        
        // Render vias on top of layers
        for geometry in &via_geometries {
            shapes.extend(geometry.to_egui_shapes());
        }
        
        // Add dimension annotations
        if self.show_dimensions {
            shapes.extend(self.create_dimension_shapes(stack, transform, viewport_rect));
        }
        
        // Add layer name labels
        if self.show_layer_names {
            shapes.extend(self.create_label_shapes(&layer_geometries, transform));
        }
        
        shapes
    }

    fn create_layer_geometries(
        &self,
        stack: &ProcessStack,
        transform: &ViewTransform,
        viewport_rect: Rect,
    ) -> Vec<LayerGeometry> {
        let mut geometries = Vec::new();
        let center_x = 0.0; // World coordinate center
        let stack_height = stack.get_total_height() as f32;
        
        // Calculate optimal layer width based on stack height and viewport
        let layer_width = calculate_optimal_layer_width(
            stack_height,
            viewport_rect.width(),
            50.0,
        );
        
        for (layer_index, layer) in stack.layers.iter().enumerate() {
            let z_bottom = layer.get_bottom_z() as f32;
            let z_top = layer.get_top_z() as f32;
            let layer_height = z_top - z_bottom;
            
            // Convert to screen coordinates
            let world_bottom = Pos2::new(center_x, -z_bottom); // Flip Y axis
            let screen_bottom = transform.world_to_screen(world_bottom);
            let screen_height = layer_height * transform.scale;
            let screen_width = layer_width * transform.scale;
            
            let is_selected = self.selected_layer.as_deref() == Some(layer.name());
            let base_color = self.color_scheme.get_layer_color(layer, layer_index);
            let alpha = self.color_scheme.get_layer_alpha(layer, is_selected);
            let color = self.color_scheme.apply_alpha(base_color, alpha);
            let outline_color = self.color_scheme.get_layer_outline_color(is_selected);
            let stroke = Stroke::new(if is_selected { 2.0 } else { 1.0 }, outline_color);
            
            let geometry = match layer {
                Layer::Conductor(conductor) => {
                    if conductor.is_trapezoid() {
                        let trapezoid = TrapezoidShape::from_conductor_layer(
                            conductor,
                            Pos2::new(screen_bottom.x, screen_bottom.y),
                            screen_width,
                            screen_height,
                            color,
                            stroke,
                        );
                        LayerGeometry::new_trapezoid(
                            layer.name().to_string(),
                            z_bottom,
                            z_top,
                            trapezoid,
                        )
                    } else {
                        let rectangle = RectangleShape::new(
                            Pos2::new(screen_bottom.x, screen_bottom.y - screen_height * 0.5),
                            screen_width,
                            screen_height,
                            color,
                            stroke,
                        );
                        LayerGeometry::new_rectangle(
                            layer.name().to_string(),
                            z_bottom,
                            z_top,
                            rectangle,
                        )
                    }
                }
                Layer::Dielectric(_) => {
                    let rectangle = RectangleShape::new(
                        Pos2::new(screen_bottom.x, screen_bottom.y - screen_height * 0.5),
                        screen_width,
                        screen_height,
                        color,
                        stroke,
                    );
                    LayerGeometry::new_rectangle(
                        layer.name().to_string(),
                        z_bottom,
                        z_top,
                        rectangle,
                    )
                }
            };
            
            geometries.push(geometry);
        }
        
        geometries
    }

    fn create_via_geometries(
        &self,
        stack: &ProcessStack,
        transform: &ViewTransform,
        _viewport_rect: Rect,
    ) -> Vec<LayerGeometry> {
        let mut geometries = Vec::new();
        let center_x = 0.0;
        
        for via in stack.via_stack.iter() {
            let z_bottom = via.get_bottom_z() as f32;
            let z_top = via.get_top_z() as f32;
            let via_height = z_top - z_bottom;
            let via_width = via.get_via_width() as f32 * 50.0; // Scale up for visibility
            
            // Convert to screen coordinates
            let world_center = Pos2::new(center_x, -(z_bottom + via_height * 0.5)); // Flip Y axis
            let screen_center = transform.world_to_screen(world_center);
            let screen_height = via_height * transform.scale;
            let screen_width = via_width * transform.scale;
            
            let via_color = self.color_scheme.get_via_color(via.get_via_type());
            let stroke = Stroke::new(1.0, Color32::BLACK);
            
            let rectangle = RectangleShape::new(
                screen_center,
                screen_width,
                screen_height,
                via_color,
                stroke,
            );
            
            let geometry = LayerGeometry::new_rectangle(
                via.name.clone(),
                z_bottom,
                z_top,
                rectangle,
            );
            
            geometries.push(geometry);
        }
        
        geometries
    }

    fn create_dimension_shapes(
        &self,
        stack: &ProcessStack,
        transform: &ViewTransform,
        viewport_rect: Rect,
    ) -> Vec<Shape> {
        let mut shapes = Vec::new();
        let margin = 20.0;
        let dimension_x = viewport_rect.max.x - margin - 60.0;
        
        for layer in &stack.layers {
            let z_bottom = layer.get_bottom_z() as f32;
            let z_top = layer.get_top_z() as f32;
            let thickness = z_top - z_bottom;
            
            let world_bottom = Pos2::new(0.0, -z_bottom);
            let world_top = Pos2::new(0.0, -z_top);
            let screen_bottom = transform.world_to_screen(world_bottom);
            let screen_top = transform.world_to_screen(world_top);
            
            // Draw dimension line
            let dim_start = Pos2::new(dimension_x, screen_bottom.y);
            let dim_end = Pos2::new(dimension_x, screen_top.y);
            
            shapes.push(Shape::line_segment(
                [dim_start, dim_end],
                Stroke::new(1.0, self.color_scheme.get_dimension_text_color()),
            ));
            
            // Draw dimension text
            let _dim_center = Pos2::new(dimension_x + 30.0, (screen_bottom.y + screen_top.y) * 0.5);
            let _thickness_text = if thickness >= 1.0 {
                format!("{thickness:.2}")
            } else if thickness >= 0.01 {
                format!("{thickness:.3}")
            } else {
                format!("{thickness:.1e}")
            };
            
            // Text rendering removed for compilation
            // shapes.push(Shape::text(...));
        }
        
        shapes
    }

    fn create_label_shapes(
        &self,
        layer_geometries: &[LayerGeometry],
        _transform: &ViewTransform,
    ) -> Vec<Shape> {
        let shapes = Vec::new();
        
        for geometry in layer_geometries {
            let bounds = geometry.get_bounds();
            let _label_pos = Pos2::new(bounds.center().x, bounds.center().y);
            
            // Only show labels for layers thick enough
            if bounds.height() > 20.0 {
                // Text rendering removed for compilation
                // shapes.push(Shape::text(...));
            }
        }
        
        shapes
    }

    pub fn set_layer_width(&mut self, width: f32) {
        self.layer_width = width.clamp(50.0, 500.0);
    }

    pub fn set_show_dimensions(&mut self, show: bool) {
        self.show_dimensions = show;
    }

    pub fn set_show_layer_names(&mut self, show: bool) {
        self.show_layer_names = show;
    }

    pub fn set_selected_layer(&mut self, layer_name: Option<String>) {
        self.selected_layer = layer_name;
    }

    pub fn get_selected_layer(&self) -> Option<&String> {
        self.selected_layer.as_ref()
    }

    pub fn hit_test(
        &self,
        stack: &ProcessStack,
        transform: &ViewTransform,
        viewport_rect: Rect,
        point: Pos2,
    ) -> Option<String> {
        let layer_geometries = self.create_layer_geometries(stack, transform, viewport_rect);
        
        // Test from top to bottom (reverse order)
        for geometry in layer_geometries.iter().rev() {
            if geometry.contains_point(point) {
                return Some(geometry.layer_name.clone());
            }
        }
        
        None
    }

    pub fn get_stack_bounds(&self, stack: &ProcessStack) -> Rect {
        if stack.layers.is_empty() {
            return Rect::NOTHING;
        }
        
        let total_height = stack.get_total_height() as f32;
        let half_width = self.layer_width * 0.5;
        
        Rect::from_min_max(
            Pos2::new(-half_width, -total_height),
            Pos2::new(half_width, 0.0),
        )
    }

    pub fn auto_fit(&self, stack: &ProcessStack, transform: &mut ViewTransform) {
        let bounds = self.get_stack_bounds(stack);
        if bounds.width() > 0.0 && bounds.height() > 0.0 {
            transform.fit_bounds(bounds, 100.0);
        }
    }
}

impl Default for StackRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for StackRenderer {
    fn clone(&self) -> Self {
        Self {
            color_scheme: ColorScheme::new(), // Create new color scheme
            layer_width: self.layer_width,
            show_dimensions: self.show_dimensions,
            show_layer_names: self.show_layer_names,
            selected_layer: self.selected_layer.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{TechnologyInfo, DielectricLayer, ConductorLayer};
    use egui::Vec2;

    fn create_test_stack() -> ProcessStack {
        let tech = TechnologyInfo::new("test_stack".to_string());
        let mut stack = ProcessStack::new(tech);
        
        stack.add_layer(Layer::Dielectric(DielectricLayer::new("oxide1".to_string(), 1.0, 4.2)));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new("metal1".to_string(), 0.5))));
        stack.add_layer(Layer::Dielectric(DielectricLayer::new("oxide2".to_string(), 1.5, 4.2)));
        
        stack
    }

    #[test]
    fn test_renderer_creation() {
        let renderer = StackRenderer::new();
        assert!(renderer.show_dimensions);
        assert!(renderer.show_layer_names);
        assert!(renderer.selected_layer.is_none());
    }

    #[test]
    fn test_layer_selection() {
        let mut renderer = StackRenderer::new();
        
        renderer.set_selected_layer(Some("metal1".to_string()));
        assert_eq!(renderer.get_selected_layer(), Some(&"metal1".to_string()));
        
        renderer.set_selected_layer(None);
        assert_eq!(renderer.get_selected_layer(), None);
    }

    #[test]
    fn test_rendering_options() {
        let mut renderer = StackRenderer::new();
        
        renderer.set_show_dimensions(false);
        assert!(!renderer.show_dimensions);
        
        renderer.set_show_layer_names(false);
        assert!(!renderer.show_layer_names);
        
        renderer.set_layer_width(300.0);
        assert_eq!(renderer.layer_width, 300.0);
        
        // Test bounds
        renderer.set_layer_width(10.0); // Too small
        assert_eq!(renderer.layer_width, 50.0);
        
        renderer.set_layer_width(1000.0); // Too large
        assert_eq!(renderer.layer_width, 500.0);
    }

    #[test]
    fn test_stack_bounds_calculation() {
        let renderer = StackRenderer::new();
        let stack = create_test_stack();
        
        let bounds = renderer.get_stack_bounds(&stack);
        assert!(bounds.width() > 0.0 && bounds.height() > 0.0);
        
        // Should span the full height of the stack
        assert_eq!(bounds.height(), stack.get_total_height() as f32);
        
        // Should be centered horizontally
        assert_eq!(bounds.center().x, 0.0);
        
        // Bottom should be at negative total height (flipped Y)
        assert_eq!(bounds.min.y, -(stack.get_total_height() as f32));
        assert_eq!(bounds.max.y, 0.0);
    }

    #[test]
    fn test_layer_geometry_creation() {
        let renderer = StackRenderer::new();
        let stack = create_test_stack();
        let transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        let viewport_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        
        let geometries = renderer.create_layer_geometries(&stack, &transform, viewport_rect);
        
        assert_eq!(geometries.len(), stack.get_layer_count());
        
        // Check that geometries are ordered from bottom to top
        for i in 1..geometries.len() {
            assert!(geometries[i].z_bottom >= geometries[i-1].z_top - 1e-6);
        }
    }

    #[test]
    fn test_render_stack() {
        let renderer = StackRenderer::new();
        let stack = create_test_stack();
        let transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        let viewport_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        
        let shapes = renderer.render_stack(&stack, &transform, viewport_rect);
        
        // Should produce shapes for layers
        assert!(!shapes.is_empty());
        
        // Should have more shapes when dimensions and labels are enabled
        let mut renderer_no_extras = renderer.clone();
        renderer_no_extras.set_show_dimensions(false);
        renderer_no_extras.set_show_layer_names(false);
        
        let shapes_minimal = renderer_no_extras.render_stack(&stack, &transform, viewport_rect);
        assert!(shapes.len() >= shapes_minimal.len());
    }

    #[test]
    fn test_auto_fit() {
        let renderer = StackRenderer::new();
        let stack = create_test_stack();
        let mut transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        
        // Initial transform
        let initial_scale = transform.scale;
        let initial_offset = transform.offset;
        
        renderer.auto_fit(&stack, &mut transform);
        
        // Should adjust scale and offset
        assert_ne!(transform.scale, initial_scale);
        assert_ne!(transform.offset, initial_offset);
        
        // Should be able to see the entire stack
        let visible_bounds = transform.get_visible_world_bounds();
        let stack_bounds = renderer.get_stack_bounds(&stack);
        assert!(visible_bounds.contains_rect(stack_bounds));
    }

    #[test]
    fn test_empty_stack() {
        let renderer = StackRenderer::new();
        let tech = TechnologyInfo::new("empty".to_string());
        let stack = ProcessStack::new(tech);
        let transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        let viewport_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        
        let shapes = renderer.render_stack(&stack, &transform, viewport_rect);
        let bounds = renderer.get_stack_bounds(&stack);
        
        // Empty stack should produce no layer shapes
        assert!(shapes.is_empty() || shapes.len() <= 2); // Maybe just background/border
        assert!(bounds.width() <= 0.0 || bounds.height() <= 0.0);
    }
}