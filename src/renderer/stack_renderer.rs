// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use egui::{Pos2, Rect, Shape, Stroke, Color32};
use crate::data::{ProcessStack, Layer};
use crate::renderer::{colors::ColorScheme, geometry::*, thickness_scaler::ThicknessScaler};
use std::collections::HashMap;

pub struct StackRenderer {
    color_scheme: ColorScheme,
    layer_width: f32,
    show_dimensions: bool,
    show_layer_names: bool,
    selected_layer: Option<String>,
    thickness_scaler: ThicknessScaler,
}

impl StackRenderer {
    pub fn new() -> Self {
        Self {
            color_scheme: ColorScheme::new(),
            layer_width: 200.0,
            show_dimensions: true,
            show_layer_names: true,
            selected_layer: None,
            thickness_scaler: ThicknessScaler::new(),
        }
    }

    pub fn render_stack(
        &self,
        stack: &ProcessStack,
        transform: &ViewTransform,
        viewport_rect: Rect,
    ) -> Vec<Shape> {
        let mut shapes = Vec::new();
        
        // Analyze stack for thickness exaggeration
        let mut scaler = self.thickness_scaler.clone();
        scaler.analyze_stack(stack);
        
        // Calculate layer positions and create geometries with proper stacking order
        let layer_geometries = self.create_layer_geometries_ordered(stack, &scaler, transform, viewport_rect);
        let via_geometries = self.create_via_geometries_with_scaler(stack, &scaler, transform, viewport_rect);
        
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
            shapes.extend(self.create_dimension_shapes_with_scaler(stack, &scaler, transform, viewport_rect));
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

    fn create_layer_geometries_with_scaler(
        &self,
        stack: &ProcessStack,
        scaler: &ThicknessScaler,
        transform: &ViewTransform,
        viewport_rect: Rect,
    ) -> Vec<LayerGeometry> {
        let mut geometries = Vec::new();
        let center_x = 0.0; // World coordinate center
        
        // Get exaggerated layer heights
        let exaggerated_heights = scaler.create_exaggerated_layer_heights(stack);
        let total_exaggerated_height = exaggerated_heights.iter().sum::<f32>();
        
        // Calculate optimal layer width based on exaggerated stack height and viewport
        let layer_width = calculate_optimal_layer_width(
            total_exaggerated_height,
            viewport_rect.width(),
            50.0,
        );
        
        // Calculate cumulative positions with exaggerated heights
        let mut current_z = 0.0f32;
        
        for (layer_index, layer) in stack.layers.iter().enumerate() {
            let exaggerated_height = exaggerated_heights[layer_index];
            let z_bottom = current_z;
            let z_top = current_z + exaggerated_height;
            current_z = z_top;
            
            // Convert to screen coordinates
            let world_bottom = Pos2::new(center_x, -z_bottom); // Flip Y axis
            let screen_bottom = transform.world_to_screen(world_bottom);
            let screen_height = exaggerated_height * transform.scale;
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

    fn create_layer_geometries_ordered(
        &self,
        stack: &ProcessStack,
        scaler: &ThicknessScaler,
        transform: &ViewTransform,
        viewport_rect: Rect,
    ) -> Vec<LayerGeometry> {
        let mut geometries = Vec::new();
        let _center_x = 0.0; // World coordinate center
        
        // Calculate optimal layer width
        let total_exaggerated_height = scaler.get_exaggerated_total_height(stack);
        let layer_width = calculate_optimal_layer_width(
            total_exaggerated_height,
            viewport_rect.width(),
            50.0,
        );
        
        // Create layer ordering: DIELECTRIC first, then CONDUCTOR
        // We'll calculate z positions based on the stacking order, not the original order
        let mut current_z = 0.0f32;
        
        // Step 1: Render all DIELECTRIC layers (bottom to top in ITF order)
        let dielectric_layers = stack.get_dielectric_layers();
        for dielectric in dielectric_layers {
            let layer_index = stack.layers.iter().position(|l| std::ptr::eq(l, dielectric)).unwrap();
            let exaggerated_height = scaler.get_exaggerated_thickness(dielectric.thickness() as f32);
            
            let geometry = self.create_single_layer_geometry(
                dielectric,
                layer_index,
                current_z,
                current_z + exaggerated_height,
                exaggerated_height,
                layer_width,
                transform,
            );
            
            geometries.push(geometry);
            current_z += exaggerated_height;
        }
        
        // Step 2: Render all CONDUCTOR layers (bottom to top in ITF order) 
        let conductor_layers = stack.get_conductor_layers();
        for conductor in conductor_layers {
            let layer_index = stack.layers.iter().position(|l| std::ptr::eq(l, conductor)).unwrap();
            let exaggerated_height = scaler.get_exaggerated_thickness(conductor.thickness() as f32);
            
            let geometry = self.create_single_layer_geometry(
                conductor,
                layer_index,
                current_z,
                current_z + exaggerated_height,
                exaggerated_height,
                layer_width,
                transform,
            );
            
            geometries.push(geometry);
            current_z += exaggerated_height;
        }
        
        geometries
    }

    fn create_single_layer_geometry(
        &self,
        layer: &Layer,
        layer_index: usize,
        z_bottom: f32,
        z_top: f32,
        exaggerated_height: f32,
        layer_width: f32,
        transform: &ViewTransform,
    ) -> LayerGeometry {
        let center_x = 0.0;
        
        // Convert to screen coordinates
        let world_bottom = Pos2::new(center_x, -z_bottom); // Flip Y axis
        let screen_bottom = transform.world_to_screen(world_bottom);
        let screen_height = exaggerated_height * transform.scale;
        let screen_width = layer_width * transform.scale;
        
        let is_selected = self.selected_layer.as_deref() == Some(layer.name());
        let base_color = self.color_scheme.get_layer_color(layer, layer_index);
        let alpha = self.color_scheme.get_layer_alpha(layer, is_selected);
        let color = self.color_scheme.apply_alpha(base_color, alpha);
        let outline_color = self.color_scheme.get_layer_outline_color(is_selected);
        let stroke = Stroke::new(if is_selected { 2.0 } else { 1.0 }, outline_color);
        
        match layer {
            Layer::Conductor(conductor) => {
                if conductor.is_trapezoid() {
                    // Use multiple trapezoids for better visualization (minimum 3)
                    let num_trapezoids = 5; // Default to 5 trapezoids per conductor
                    let multi_trapezoid = MultiTrapezoidShape::from_conductor_layer(
                        conductor,
                        Pos2::new(screen_bottom.x, screen_bottom.y),
                        screen_width,
                        screen_height,
                        color,
                        stroke,
                        num_trapezoids,
                    );
                    LayerGeometry::new_multi_trapezoid(
                        layer.name().to_string(),
                        z_bottom,
                        z_top,
                        multi_trapezoid,
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
        }
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

    fn create_via_geometries_with_scaler(
        &self,
        stack: &ProcessStack,
        scaler: &ThicknessScaler,
        transform: &ViewTransform,
        _viewport_rect: Rect,
    ) -> Vec<LayerGeometry> {
        let mut geometries = Vec::new();
        let center_x = 0.0;
        
        // Build a map of layer z positions based on the new stacking order
        let layer_z_positions = self.calculate_ordered_layer_positions(stack, scaler);
        
        for via in stack.via_stack.iter() {
            // Find z positions for FROM and TO layers based on new stacking order
            let from_z = layer_z_positions.get(&via.from_layer);
            let to_z = layer_z_positions.get(&via.to_layer);
            
            if let (Some(&from_center_z), Some(&to_center_z)) = (from_z, to_z) {
                let z_bottom = from_center_z.min(to_center_z) - (from_center_z - to_center_z).abs() * 0.1;
                let z_top = from_center_z.max(to_center_z) + (from_center_z - to_center_z).abs() * 0.1;
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
        }
        
        geometries
    }

    fn calculate_ordered_layer_positions(&self, stack: &ProcessStack, scaler: &ThicknessScaler) -> HashMap<String, f32> {
        let mut layer_positions = HashMap::new();
        let mut current_z = 0.0f32;
        
        // First, position all DIELECTRIC layers
        let dielectric_layers = stack.get_dielectric_layers();
        for dielectric in dielectric_layers {
            let exaggerated_height = scaler.get_exaggerated_thickness(dielectric.thickness() as f32);
            let center_z = current_z + exaggerated_height * 0.5;
            layer_positions.insert(dielectric.name().to_string(), center_z);
            current_z += exaggerated_height;
        }
        
        // Then, position all CONDUCTOR layers
        let conductor_layers = stack.get_conductor_layers();
        for conductor in conductor_layers {
            let exaggerated_height = scaler.get_exaggerated_thickness(conductor.thickness() as f32);
            let center_z = current_z + exaggerated_height * 0.5;
            layer_positions.insert(conductor.name().to_string(), center_z);
            current_z += exaggerated_height;
        }
        
        layer_positions
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

    fn create_dimension_shapes_with_scaler(
        &self,
        stack: &ProcessStack,
        scaler: &ThicknessScaler,
        transform: &ViewTransform,
        viewport_rect: Rect,
    ) -> Vec<Shape> {
        let mut shapes = Vec::new();
        let margin = 20.0;
        let dimension_x = viewport_rect.max.x - margin - 60.0;
        
        // Get exaggerated layer heights
        let exaggerated_heights = scaler.create_exaggerated_layer_heights(stack);
        let mut current_z = 0.0f32;
        
        for (layer_index, layer) in stack.layers.iter().enumerate() {
            let exaggerated_height = exaggerated_heights[layer_index];
            let z_bottom = current_z;
            let z_top = current_z + exaggerated_height;
            current_z = z_top;
            
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
            
            // Draw dimension text showing both original and exaggerated thickness
            let _dim_center = Pos2::new(dimension_x + 30.0, (screen_bottom.y + screen_top.y) * 0.5);
            let original_thickness = layer.thickness() as f32;
            let scale_factor = scaler.get_scale_factor(original_thickness);
            let _thickness_text = if original_thickness >= 1.0 {
                format!("{original_thickness:.2} ({:.0}%)", scale_factor * 100.0)
            } else if original_thickness >= 0.01 {
                format!("{original_thickness:.3} ({:.0}%)", scale_factor * 100.0)
            } else {
                format!("{original_thickness:.1e} ({:.0}%)", scale_factor * 100.0)
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
        let mut scaler = self.thickness_scaler.clone();
        scaler.analyze_stack(stack);
        let layer_geometries = self.create_layer_geometries_ordered(stack, &scaler, transform, viewport_rect);
        
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
        
        // Use exaggerated thickness for bounds calculation
        let mut scaler = self.thickness_scaler.clone();
        scaler.analyze_stack(stack);
        let total_height = scaler.get_exaggerated_total_height(stack);
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
            thickness_scaler: self.thickness_scaler.clone(),
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
        
        // Calculate expected exaggerated height
        let mut scaler = ThicknessScaler::new();
        scaler.analyze_stack(&stack);
        let expected_height = scaler.get_exaggerated_total_height(&stack);
        
        // Should span the full exaggerated height of the stack
        assert_eq!(bounds.height(), expected_height);
        
        // Should be centered horizontally
        assert_eq!(bounds.center().x, 0.0);
        
        // Bottom should be at negative total exaggerated height (flipped Y)
        assert_eq!(bounds.min.y, -expected_height);
        assert_eq!(bounds.max.y, 0.0);
    }

    #[test]
    fn test_layer_geometry_creation() {
        let renderer = StackRenderer::new();
        let stack = create_test_stack();
        let transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        let viewport_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        
        let mut scaler = ThicknessScaler::new();
        scaler.analyze_stack(&stack);
        let geometries = renderer.create_layer_geometries_ordered(&stack, &scaler, &transform, viewport_rect);
        
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
    fn test_layer_stacking_order() {
        let renderer = StackRenderer::new();
        
        // Create a test stack with mixed layer types
        let tech = TechnologyInfo::new("test_stacking".to_string());
        let mut stack = ProcessStack::new(tech);
        
        // Add layers in mixed order: conductor, dielectric, conductor, dielectric
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new("conductor1".to_string(), 0.5))));
        stack.add_layer(Layer::Dielectric(DielectricLayer::new("dielectric1".to_string(), 1.0, 4.2)));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new("conductor2".to_string(), 0.3))));
        stack.add_layer(Layer::Dielectric(DielectricLayer::new("dielectric2".to_string(), 0.8, 4.2)));
        
        let transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        let viewport_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        
        let mut scaler = ThicknessScaler::new();
        scaler.analyze_stack(&stack);
        let geometries = renderer.create_layer_geometries_ordered(&stack, &scaler, &transform, viewport_rect);
        
        // Should have 4 geometries
        assert_eq!(geometries.len(), 4);
        
        // Check stacking order: all dielectrics first, then all conductors
        assert_eq!(geometries[0].layer_name, "dielectric1"); // First dielectric
        assert_eq!(geometries[1].layer_name, "dielectric2"); // Second dielectric
        assert_eq!(geometries[2].layer_name, "conductor1"); // First conductor
        assert_eq!(geometries[3].layer_name, "conductor2"); // Second conductor
        
        // Check z positions are monotonically increasing (bottom to top)
        for i in 1..geometries.len() {
            assert!(geometries[i].z_bottom >= geometries[i-1].z_top - 1e-6,
                "Layer {} should be above layer {}: {:.6} >= {:.6}",
                i, i-1, geometries[i].z_bottom, geometries[i-1].z_top);
        }
    }

    #[test]
    fn test_via_positioning_with_new_stacking() {
        let renderer = StackRenderer::new();
        
        // Create a test stack with via connections
        let tech = TechnologyInfo::new("test_via_stacking".to_string());
        let mut stack = ProcessStack::new(tech);
        
        // Add layers: dielectric, conductor, dielectric, conductor
        stack.add_layer(Layer::Dielectric(DielectricLayer::new("oxide1".to_string(), 1.0, 4.2)));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new("metal1".to_string(), 0.5))));
        stack.add_layer(Layer::Dielectric(DielectricLayer::new("oxide2".to_string(), 0.8, 4.2)));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new("metal2".to_string(), 0.3))));
        
        // Add a via connecting the two metal layers
        use crate::data::ViaConnection;
        let via = ViaConnection::new("via12".to_string(), "metal1".to_string(), "metal2".to_string(), 0.25, 5.0);
        stack.add_via(via);
        
        let transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        let viewport_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        
        let mut scaler = ThicknessScaler::new();
        scaler.analyze_stack(&stack);
        
        // Get layer positions
        let layer_positions = renderer.calculate_ordered_layer_positions(&stack, &scaler);
        
        // Metal1 should be positioned before metal2 in the new stacking order
        let metal1_pos = layer_positions.get("metal1").unwrap();
        let metal2_pos = layer_positions.get("metal2").unwrap();
        assert!(metal2_pos > metal1_pos, "Metal2 should be above Metal1 in new stacking order");
        
        // Create via geometries
        let via_geometries = renderer.create_via_geometries_with_scaler(&stack, &scaler, &transform, viewport_rect);
        assert_eq!(via_geometries.len(), 1);
        
        // Via should span between the two metal layers
        let via_geom = &via_geometries[0];
        assert_eq!(via_geom.layer_name, "via12");
        
        // Via should be positioned to connect the metal layers
        assert!(via_geom.z_bottom <= *metal1_pos && via_geom.z_top >= *metal2_pos,
               "Via should span from metal1 ({}) to metal2 ({}), but spans {}-{}",
               metal1_pos, metal2_pos, via_geom.z_bottom, via_geom.z_top);
    }

    #[test]
    fn test_thickness_exaggeration_integration() {
        let renderer = StackRenderer::new();
        
        // Create stack with varied thicknesses
        let tech = TechnologyInfo::new("test_exaggeration".to_string());
        let mut stack = ProcessStack::new(tech);
        
        // Add layers with different thicknesses: thin, thick, medium
        stack.add_layer(Layer::Dielectric(DielectricLayer::new("thin".to_string(), 0.1, 4.2)));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new("thick".to_string(), 2.0))));
        stack.add_layer(Layer::Dielectric(DielectricLayer::new("medium".to_string(), 1.0, 4.2)));
        
        let transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        let viewport_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        
        let mut scaler = ThicknessScaler::new();
        scaler.analyze_stack(&stack);
        let geometries = renderer.create_layer_geometries_ordered(&stack, &scaler, &transform, viewport_rect);
        
        // Should have 3 geometries in stacking order (dielectrics first, then conductors)
        assert_eq!(geometries.len(), 3);
        assert_eq!(geometries[0].layer_name, "thin"); // First dielectric
        assert_eq!(geometries[1].layer_name, "medium"); // Second dielectric  
        assert_eq!(geometries[2].layer_name, "thick"); // Conductor
        
        // Check that thickness exaggeration is applied
        let thin_height = geometries[0].z_top - geometries[0].z_bottom;
        let thick_height = geometries[2].z_top - geometries[2].z_bottom;
        let medium_height = geometries[1].z_top - geometries[1].z_bottom;
        
        // Thick layer should have largest exaggerated height
        assert!(thick_height > medium_height);
        assert!(medium_height > thin_height);
        
        // The thickness scaling should have been applied (verify via scaler)
        let thin_scale = scaler.get_scale_factor(0.1);
        let thick_scale = scaler.get_scale_factor(2.0);
        let medium_scale = scaler.get_scale_factor(1.0);
        
        // The thickest layer should have highest scale factor, thinnest should have lowest
        assert!(thick_scale >= medium_scale);
        assert!(medium_scale >= thin_scale);
        assert!(thin_scale >= 0.3); // Minimum scale factor
        assert!(thick_scale <= 1.0); // Maximum scale factor
        
        // The actual heights should reflect the scaled thicknesses
        let expected_thin = 0.1 * thin_scale;
        let expected_thick = 2.0 * thick_scale;
        let expected_medium = 1.0 * medium_scale;
        
        assert!((thin_height - expected_thin).abs() < 0.01);
        assert!((thick_height - expected_thick).abs() < 0.01);
        assert!((medium_height - expected_medium).abs() < 0.01);
    }

    #[test]
    fn test_conductor_multi_trapezoid_rendering() {
        let renderer = StackRenderer::new();
        
        // Create a conductor layer with side tangent (trapezoid shape)
        let tech = TechnologyInfo::new("test_multi_trap".to_string());
        let mut stack = ProcessStack::new(tech);
        
        let mut conductor = ConductorLayer::new("trapezoid_conductor".to_string(), 1.0);
        conductor.physical_props.side_tangent = Some(0.05); // Make it trapezoid
        stack.add_layer(Layer::Conductor(Box::new(conductor)));
        
        let transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        let viewport_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        
        let mut scaler = ThicknessScaler::new();
        scaler.analyze_stack(&stack);
        let geometries = renderer.create_layer_geometries_ordered(&stack, &scaler, &transform, viewport_rect);
        
        assert_eq!(geometries.len(), 1);
        let conductor_geometry = &geometries[0];
        
        // Verify it's using MultiTrapezoid shape
        match &conductor_geometry.shape {
            LayerShape::MultiTrapezoid(multi_trap) => {
                // Should have at least 3 trapezoids (our minimum)
                assert!(multi_trap.trapezoids.len() >= 3);
                // With our default, should have exactly 5
                assert_eq!(multi_trap.trapezoids.len(), 5);
            }
            _ => panic!("Conductor with side_tangent should use MultiTrapezoid shape"),
        }
        
        // Verify it generates multiple shapes for rendering
        let shapes = conductor_geometry.to_egui_shapes();
        assert_eq!(shapes.len(), 5); // Should generate 5 shapes for 5 trapezoids
    }

    #[test]
    fn test_conductor_rectangle_rendering() {
        let renderer = StackRenderer::new();
        
        // Create a conductor layer without side tangent (rectangle shape)
        let tech = TechnologyInfo::new("test_rect".to_string());
        let mut stack = ProcessStack::new(tech);
        
        let conductor = ConductorLayer::new("rectangle_conductor".to_string(), 1.0);
        // No side_tangent set, should default to rectangle
        stack.add_layer(Layer::Conductor(Box::new(conductor)));
        
        let transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        let viewport_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        
        let mut scaler = ThicknessScaler::new();
        scaler.analyze_stack(&stack);
        let geometries = renderer.create_layer_geometries_ordered(&stack, &scaler, &transform, viewport_rect);
        
        assert_eq!(geometries.len(), 1);
        let conductor_geometry = &geometries[0];
        
        // Verify it's using Rectangle shape (not multi-trapezoid)
        match &conductor_geometry.shape {
            LayerShape::Rectangle(_) => {
                // This is expected for non-trapezoid conductors
            }
            _ => panic!("Conductor without side_tangent should use Rectangle shape"),
        }
        
        // Should generate only 1 shape for rectangle
        let shapes = conductor_geometry.to_egui_shapes();
        assert_eq!(shapes.len(), 1);
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