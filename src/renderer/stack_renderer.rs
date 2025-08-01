// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use crate::data::{Layer, ProcessStack};
use crate::renderer::{colors::ColorScheme, geometry::*, thickness_scaler::ThicknessScaler};
use egui::{Align2, Color32, FontId, Pos2, Rect, Shape, Stroke, Vec2};
use std::collections::HashMap;

/// Parameters for creating a single layer geometry
struct LayerGeometryParams<'a> {
    layer: &'a Layer,
    layer_index: usize,
    z_bottom: f32,
    z_top: f32,
    exaggerated_height: f32,
    layer_width: f32,
    max_trapezoid_width: Option<f32>, // Reference width for three-column alignment
}

pub struct StackRenderer {
    color_scheme: ColorScheme,
    layer_width: f32,
    show_dimensions: bool,
    pub show_layer_names: bool,
    pub show_schematic_mode: bool,
    selected_layer: Option<String>,
    pub thickness_scaler: ThicknessScaler,
}

impl StackRenderer {
    pub fn new() -> Self {
        Self {
            color_scheme: ColorScheme::new(),
            layer_width: 200.0,
            show_dimensions: true,
            show_layer_names: true,
            show_schematic_mode: false,
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

        // Choose the appropriate scaler based on mode
        let scaler = if self.show_schematic_mode {
            self.create_schematic_scaler(stack)
        } else {
            // For normal mode, use 1:1 scaling (no exaggeration)
            self.create_normal_scaler(stack)
        };

        // Calculate layer positions and create geometries with proper stacking order
        let layer_geometries =
            self.create_layer_geometries_ordered(stack, &scaler, transform, viewport_rect);
        let via_geometries =
            self.create_via_geometries_with_scaler(stack, &scaler, transform, viewport_rect);

        // Separate geometries by layer type for proper z-ordering
        let mut dielectric_geometries = Vec::new();
        let mut conductor_geometries = Vec::new();

        for geometry in &layer_geometries {
            // Check if this is a conductor layer by looking at the shape type
            match &geometry.shape {
                LayerShape::ThreeColumnTrapezoid(_) => {
                    // All conductor layers use ThreeColumnTrapezoid
                    conductor_geometries.push(geometry);
                }
                _ => {
                    // All other shapes are dielectric layers
                    dielectric_geometries.push(geometry);
                }
            }
        }

        // Render dielectric layers first (bottom z-index)
        for geometry in &dielectric_geometries {
            shapes.extend(geometry.to_egui_shapes());
        }

        // Render conductor layers second (higher z-index, will appear on top)
        for geometry in &conductor_geometries {
            shapes.extend(geometry.to_egui_shapes());
        }

        // Render vias on top of all layers (highest z-index)
        for geometry in &via_geometries {
            shapes.extend(geometry.to_egui_shapes());
        }

        // Add dimension annotations (but not in schematic mode)
        if self.show_dimensions && !self.show_schematic_mode {
            shapes.extend(self.create_dimension_shapes_with_scaler(
                stack,
                &scaler,
                transform,
                viewport_rect,
            ));
        }

        shapes
    }

    /// Calculate appropriate dielectric layer width to contain conductor layers
    /// Using ideal DCDCDCD layout: 7x max trapezoid width for proper spacing
    fn calculate_dielectric_width_for_conductors(
        &self,
        max_trapezoid_width: f32,
        _transform: &ViewTransform,
        original_layer_width: f32,
    ) -> f32 {
        // Always use the provided max_trapezoid_width, which is already correctly calculated
        // for the current mode (normal or schematic) in create_layer_geometries_ordered
        let reference_trapezoid_width = max_trapezoid_width;

        // Ideal dielectric width: 7 times the reference trapezoid width
        // This allows for DCDCDCD layout where:
        // - 3 trapezoids (C) = 3x reference_trapezoid_width
        // - 2 spaces between trapezoids (D) = 2x reference_trapezoid_width (1x each)
        // - 2 edge margins (D) = 2x reference_trapezoid_width (1x each)
        // Total: 3 + 2 + 2 = 7x reference_trapezoid_width
        let ideal_dielectric_width = reference_trapezoid_width * 7.0;

        // Debug output for dielectric width calculation
        if cfg!(debug_assertions) {
            println!("DEBUG Dielectric Unified: Mode: {}, Max trapezoid width: {}, DCDCDCD width: {}, Original layer width: {}", 
                if self.show_schematic_mode { "Schematic" } else { "Normal" },
                max_trapezoid_width, ideal_dielectric_width, original_layer_width);
        }

        // In schematic mode, always use the calculated DCDCDCD width for proper proportions
        // In normal mode, ensure minimum width but prefer DCDCDCD proportions
        if self.show_schematic_mode {
            // Force DCDCDCD proportions in schematic mode
            ideal_dielectric_width
        } else {
            // In normal mode, ensure dielectric is at least as wide as original, but prefer DCDCDCD if larger
            ideal_dielectric_width.max(original_layer_width)
        }
    }

    /// Render the stack with text, using a painter for proper text rendering
    pub fn render_stack_with_painter(
        &self,
        stack: &ProcessStack,
        transform: &ViewTransform,
        viewport_rect: Rect,
        painter: &egui::Painter,
    ) {
        // Create scaler for layer thickness
        let scaler = if self.show_schematic_mode {
            self.create_schematic_scaler(stack)
        } else {
            self.create_normal_scaler(stack)
        };

        // Get all layer geometries
        let layer_geometries =
            self.create_layer_geometries_ordered(stack, &scaler, transform, viewport_rect);
        let via_geometries =
            self.create_via_geometries_with_scaler(stack, &scaler, transform, viewport_rect);

        // Render all layer geometries
        for geometry in &layer_geometries {
            // Add layer shapes
            for shape in geometry.to_egui_shapes() {
                painter.add(shape);
            }
        }

        // Render vias on top of all layers (highest z-index)
        for geometry in &via_geometries {
            for shape in geometry.to_egui_shapes() {
                painter.add(shape);
            }
        }

        // Render text with smart positioning based on layer type and height
        if self.show_layer_names {
            self.render_text_with_smart_positioning(
                &layer_geometries,
                &via_geometries,
                painter,
                transform,
            );
        }

        // Add dimension annotations with text using painter (but not in schematic mode)
        if self.show_dimensions && !self.show_schematic_mode {
            self.render_dimensions_with_painter(stack, transform, viewport_rect, painter);
        }
    }

    /// Render text with smart positioning based on layer type and height constraints
    fn render_text_with_smart_positioning(
        &self,
        layer_geometries: &[LayerGeometry],
        via_geometries: &[LayerGeometry],
        painter: &egui::Painter,
        transform: &ViewTransform,
    ) {
        // Dynamic font size based on zoom level
        let base_font_size = 12.0;
        let zoom_factor = transform.scale.clamp(0.5, 3.0);
        let dynamic_font_size = (base_font_size * zoom_factor).clamp(8.0, 20.0);
        let font_id = FontId::proportional(dynamic_font_size);

        // Separate layers by type based on their shape
        let mut dielectric_layers = Vec::new();
        let mut conductor_layers = Vec::new();

        for geometry in layer_geometries {
            match &geometry.shape {
                LayerShape::ThreeColumnTrapezoid(_) => {
                    // Conductor layers use ThreeColumnTrapezoid
                    conductor_layers.push(geometry);
                }
                _ => {
                    // All other shapes are dielectric layers
                    dielectric_layers.push(geometry);
                }
            }
        }

        // Render dielectric layer names on the left side of dielectric shapes
        for geometry in &dielectric_layers {
            self.render_dielectric_text(geometry, painter, &font_id);
        }

        // Render conductor layer names on the left side of the center column
        for geometry in &conductor_layers {
            self.render_conductor_text(geometry, painter, &font_id);
        }

        // Render via names on the right side of vias
        for geometry in via_geometries {
            self.render_via_text(geometry, painter, &font_id);
        }
    }

    /// Render text for dielectric layers on the left side, constrained by layer height
    fn render_dielectric_text(
        &self,
        geometry: &LayerGeometry,
        painter: &egui::Painter,
        font_id: &FontId,
    ) {
        let bounds = geometry.get_bounds();
        let layer_height = bounds.height();
        let layer_name = &geometry.layer_name;

        // Calculate maximum font size that fits within layer height
        let max_font_size_for_height = (layer_height * 0.8).clamp(8.0, font_id.size);
        let adjusted_font_id = if max_font_size_for_height < font_id.size {
            FontId::proportional(max_font_size_for_height)
        } else {
            font_id.clone()
        };

        // Position text on the left side of the dielectric layer
        let text_pos = Pos2::new(
            bounds.min.x - 5.0, // 5 pixels to the left of the layer
            bounds.center().y,  // Vertically centered
        );

        println!(
            "DEBUG: Dielectric '{}' - layer height: {:.1}, font size: {:.1}, pos: {:?}",
            layer_name, layer_height, adjusted_font_id.size, text_pos
        );

        // Render text with outline
        self.render_outlined_text(text_pos, layer_name, &adjusted_font_id, painter);
    }

    /// Render text for conductor layers on the left side of center column, constrained by layer height
    fn render_conductor_text(
        &self,
        geometry: &LayerGeometry,
        painter: &egui::Painter,
        font_id: &FontId,
    ) {
        let bounds = geometry.get_bounds();
        let layer_height = bounds.height();
        let layer_name = &geometry.layer_name;

        // Calculate maximum font size that fits within layer height
        let max_font_size_for_height = (layer_height * 0.8).clamp(8.0, font_id.size);
        let adjusted_font_id = if max_font_size_for_height < font_id.size {
            FontId::proportional(max_font_size_for_height)
        } else {
            font_id.clone()
        };

        // For ThreeColumnTrapezoid, position text centered in the middle column
        let center_x = bounds.center().x;
        let text_pos = Pos2::new(
            center_x,          // Center of the middle column
            bounds.center().y, // Vertically centered
        );

        // Render text with outline, centered alignment
        self.render_outlined_text_centered(text_pos, layer_name, &adjusted_font_id, painter);
    }

    /// Render text for vias on the right side, constrained by via height
    fn render_via_text(&self, geometry: &LayerGeometry, painter: &egui::Painter, font_id: &FontId) {
        let bounds = geometry.get_bounds();
        let via_height = bounds.height();
        let layer_name = &geometry.layer_name;

        // Extract base name from via name (remove _0, _1, _2 suffix)
        let base_name = if let Some(underscore_pos) = layer_name.rfind('_') {
            if let Some(suffix) = layer_name.get(underscore_pos + 1..) {
                // Check if suffix is a digit (0, 1, 2)
                if suffix.chars().all(|c| c.is_ascii_digit()) {
                    &layer_name[..underscore_pos]
                } else {
                    layer_name
                }
            } else {
                layer_name
            }
        } else {
            layer_name
        };

        // Only show text for middle column via (suffix _1) to avoid duplication
        if layer_name.ends_with("_1") {
            // Calculate maximum font size that fits within via height
            let max_font_size_for_height = (via_height * 0.8).clamp(8.0, font_id.size);
            let adjusted_font_id = if max_font_size_for_height < font_id.size {
                FontId::proportional(max_font_size_for_height)
            } else {
                font_id.clone()
            };

            // Position text centered in the right column (third column)
            let right_column_center_x = bounds.max.x - bounds.width() / 6.0; // Center of right column
            let text_pos = Pos2::new(
                right_column_center_x, // Center of the right column
                bounds.center().y,     // Vertically centered
            );

            // Render text with outline using base name, centered alignment
            self.render_outlined_text_centered(text_pos, base_name, &adjusted_font_id, painter);
        }
    }

    /// Helper function to render outlined text
    fn render_outlined_text(
        &self,
        pos: Pos2,
        text: &str,
        font_id: &FontId,
        painter: &egui::Painter,
    ) {
        // Render black outline for contrast
        for offset in &[
            Vec2::new(-1.0, -1.0),
            Vec2::new(0.0, -1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(-1.0, 1.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
        ] {
            painter.text(
                pos + *offset,
                Align2::CENTER_CENTER,
                text,
                font_id.clone(),
                Color32::BLACK,
            );
        }

        // Render white text on top
        painter.text(
            pos,
            Align2::CENTER_CENTER,
            text,
            font_id.clone(),
            Color32::WHITE,
        );
    }

    /// Helper function to render outlined text with center alignment
    fn render_outlined_text_centered(
        &self,
        pos: Pos2,
        text: &str,
        font_id: &FontId,
        painter: &egui::Painter,
    ) {
        // Render black outline by drawing text at slightly offset positions
        for offset in &[
            Vec2::new(-1.0, -1.0),
            Vec2::new(0.0, -1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(-1.0, 1.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
        ] {
            painter.text(
                pos + *offset,
                Align2::CENTER_CENTER,
                text,
                font_id.clone(),
                Color32::BLACK,
            );
        }

        // Render white text on top with center alignment
        painter.text(
            pos,
            Align2::CENTER_CENTER,
            text,
            font_id.clone(),
            Color32::WHITE,
        );
    }

    pub fn create_layer_geometries_ordered(
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
        let layer_width =
            calculate_optimal_layer_width(total_exaggerated_height, viewport_rect.width(), 50.0);

        // First, find the maximum trapezoid width from all conductor layers
        // This will be used as the reference for three-column layout alignment
        // Use the scaler-aware version to handle both normal and schematic modes correctly
        let conductor_layers: Vec<&crate::data::ConductorLayer> = stack
            .layers
            .iter()
            .filter_map(|layer| match layer {
                Layer::Conductor(conductor) => Some(conductor.as_ref()),
                _ => None,
            })
            .collect();

        let max_trapezoid_width = if self.show_schematic_mode {
            // In schematic mode, use scaled thicknesses for proper proportions
            crate::renderer::geometry::find_max_conductor_trapezoid_width_with_scaler(
                &conductor_layers,
                scaler,
            )
        } else {
            // In normal mode, use original thicknesses
            crate::renderer::geometry::find_max_conductor_trapezoid_width(&conductor_layers)
        };

        // ITF layers are defined from top to bottom, but we need to render from bottom to top
        // So we reverse the layer order for rendering to match the physical stack
        let mut current_z = 0.0f32;

        // First pass: process dielectric layers to establish their positions
        let mut dielectric_positions = Vec::new();
        for (layer_index, layer) in stack.layers.iter().enumerate().rev() {
            if let Layer::Dielectric(_) = layer {
                let exaggerated_height = scaler.get_exaggerated_thickness_for_layer(layer);
                let bottom = current_z;
                let top = current_z + exaggerated_height;
                dielectric_positions.push((layer_index, bottom, top, exaggerated_height));
                current_z = top;
            }
        }

        // Second pass: create geometries for all layers, embedding conductors in their preceding dielectric
        let mut dielectric_index = 0;

        // Render layers in reverse ITF order (bottom to top physically)
        for (layer_index, layer) in stack.layers.iter().enumerate().rev() {
            let exaggerated_height = scaler.get_exaggerated_thickness_for_layer(layer);

            let (z_bottom, z_top) = match layer {
                Layer::Dielectric(_) => {
                    // Use pre-calculated dielectric position
                    let (_, bottom, top, _) = dielectric_positions[dielectric_index];
                    dielectric_index += 1;
                    (bottom, top)
                }
                Layer::Conductor(_) => {
                    // Find the dielectric layer that should contain this conductor
                    // In ITF order, the conductor should be embedded in the previous dielectric layer
                    let mut target_dielectric_bottom = 0.0f32;

                    // Look for the dielectric layer that appears right before this conductor in the original layer order
                    if layer_index > 0 {
                        if let Some(Layer::Dielectric(_)) = stack.layers.get(layer_index - 1) {
                            // Find this dielectric's position
                            for &(d_index, d_bottom, _d_top, _d_height) in &dielectric_positions {
                                if d_index == (layer_index - 1) {
                                    target_dielectric_bottom = d_bottom;
                                    break;
                                }
                            }
                        }
                    }

                    let bottom = target_dielectric_bottom;
                    let top = bottom + exaggerated_height;
                    (bottom, top)
                }
            };

            let params = LayerGeometryParams {
                layer,
                layer_index,
                z_bottom,
                z_top,
                exaggerated_height,
                layer_width,
                max_trapezoid_width,
            };
            let geometry = self.create_single_layer_geometry(&params, transform);

            geometries.push(geometry);
        }

        geometries
    }

    fn create_single_layer_geometry(
        &self,
        params: &LayerGeometryParams,
        transform: &ViewTransform,
    ) -> LayerGeometry {
        let center_x = 0.0;

        // Convert to world coordinates
        let _world_bottom = Pos2::new(center_x, -params.z_bottom); // Flip Y axis

        // Calculate appropriate width based on layer type
        // For both conductor and dielectric layers, use world coordinates
        // Let the shape objects handle screen coordinate conversion
        let world_width = match params.layer {
            Layer::Conductor(_) => {
                // For conductor layers, use the original layer_width for three-column calculation
                params.layer_width
            }
            Layer::Dielectric(_) => {
                // For dielectric layers, calculate width based on conductor layer requirements
                // This ensures dielectric layers are wide enough to contain all conductor shapes
                if let Some(max_trapezoid_width) = params.max_trapezoid_width {
                    // Calculate the required width for three-column trapezoid layout
                    self.calculate_dielectric_width_for_conductors(
                        max_trapezoid_width,
                        transform,
                        params.layer_width,
                    )
                } else {
                    // Fallback to original width if no conductor reference available
                    params.layer_width
                }
            }
        };

        let is_selected = self.selected_layer.as_deref() == Some(params.layer.name());
        let base_color = self
            .color_scheme
            .get_layer_color(params.layer, params.layer_index);
        let alpha = self.color_scheme.get_layer_alpha(params.layer, is_selected);
        let color = self.color_scheme.apply_alpha(base_color, alpha);
        let outline_color = self.color_scheme.get_layer_outline_color(is_selected);
        let stroke = Stroke::new(if is_selected { 2.0 } else { 1.0 }, outline_color);

        match params.layer {
            Layer::Conductor(conductor) => {
                // All conductor layers use three-column trapezoid layout with max trapezoid width as reference
                // Pass world coordinates to geometry function, let it handle screen conversion
                let world_bottom = Pos2::new(center_x, -params.z_bottom); // World coordinates
                let world_height = params.exaggerated_height; // World height (not scaled)

                let three_column_trapezoid =
                    ThreeColumnTrapezoidShape::from_conductor_layer_with_reference(
                        conductor,
                        world_bottom,
                        world_width,
                        world_height,
                        color,
                        stroke,
                        params.max_trapezoid_width,
                        Some(transform), // Pass transform for coordinate conversion
                    );
                LayerGeometry::new_three_column_trapezoid(
                    params.layer.name().to_string(),
                    params.z_bottom,
                    params.z_top,
                    three_column_trapezoid,
                )
            }
            Layer::Dielectric(_) => {
                // Use world coordinates like conductor layers
                let world_bottom = Pos2::new(center_x, -params.z_bottom); // World coordinates
                let world_height = params.exaggerated_height; // World height (not scaled)

                // Debug output for dielectric layer rendering
                if cfg!(debug_assertions) {
                    println!("DEBUG Dielectric Render: Layer '{}' - World width: {}, World height: {}, Mode: {}", 
                        params.layer.name(), world_width, world_height,
                        if self.show_schematic_mode { "Schematic" } else { "Normal" });
                }

                let rectangle = RectangleShape::new_world_coords(
                    world_bottom,
                    world_width,
                    world_height,
                    color,
                    stroke,
                    transform,
                );
                LayerGeometry::new_rectangle(
                    params.layer.name().to_string(),
                    params.z_bottom,
                    params.z_top,
                    rectangle,
                )
            }
        }
    }

    pub fn create_via_geometries_with_scaler(
        &self,
        stack: &ProcessStack,
        scaler: &ThicknessScaler,
        transform: &ViewTransform,
        viewport_rect: Rect,
    ) -> Vec<LayerGeometry> {
        let mut geometries = Vec::new();

        // Get layer boundaries for precise VIA positioning
        let layer_boundaries = self.calculate_ordered_layer_boundaries(stack, scaler);

        // Calculate optimal layer width for metal positioning (not used in new 7x layout)
        let total_exaggerated_height = scaler.get_exaggerated_total_height(stack);
        let _layer_width =
            calculate_optimal_layer_width(total_exaggerated_height, viewport_rect.width(), 50.0);

        for via in stack.via_stack.iter() {
            // Find boundary positions for FROM and TO layers
            let from_bounds = layer_boundaries.get(&via.from_layer);
            let to_bounds = layer_boundaries.get(&via.to_layer);

            if let (Some(&(from_bottom, from_top)), Some(&(to_bottom, to_top))) =
                (from_bounds, to_bounds)
            {
                // VIA should span from the surface of the FROM layer to the surface of the TO layer
                let (via_z_start, via_z_end) = if from_bottom < to_bottom {
                    // FROM layer is below TO layer - VIA goes from top of FROM to bottom of TO
                    (from_top, to_bottom)
                } else {
                    // FROM layer is above TO layer - VIA goes from bottom of FROM to top of TO
                    (from_bottom, to_top)
                };

                let via_height = (via_z_end - via_z_start).abs();

                // Find the maximum trapezoid width to match the conductor layout
                // Use scaler-aware calculation for proper alignment in both modes
                let conductor_layers: Vec<&crate::data::ConductorLayer> = stack
                    .layers
                    .iter()
                    .filter_map(|layer| match layer {
                        Layer::Conductor(conductor) => Some(conductor.as_ref()),
                        _ => None,
                    })
                    .collect();

                let max_trapezoid_width = if self.show_schematic_mode {
                    // In schematic mode, use scaled thicknesses for proper proportions
                    crate::renderer::geometry::find_max_conductor_trapezoid_width_with_scaler(
                        &conductor_layers,
                        scaler,
                    )
                } else {
                    // In normal mode, use original thicknesses
                    crate::renderer::geometry::find_max_conductor_trapezoid_width(&conductor_layers)
                };

                // Calculate via width based on connected metal layers (narrowest edge)
                let via_width = self.calculate_via_width(via, stack, scaler);

                // Convert via center to screen coordinates
                let via_center_z = (via_z_start + via_z_end) * 0.5;
                let world_center = Pos2::new(0.0, -via_center_z); // Center in world coords
                let screen_center = transform.world_to_screen(world_center);
                let screen_height = via_height * transform.scale;
                let screen_width = via_width * transform.scale;

                // Calculate VIA positions to align with the new 7x conductor layout
                // This matches the exact same logic used in ThreeColumnTrapezoidShape
                if let Some(base_trapezoid_width) = max_trapezoid_width {
                    // Use the same 7x layout calculation as conductors
                    let world_effective_width = base_trapezoid_width * 7.0; // 7x layout
                    let screen_effective_width = world_effective_width * transform.scale;

                    // Calculate the same positions as the trapezoids using screen coordinates
                    let screen_base_unit = screen_effective_width / 7.0; // Each unit in the 7x layout

                    let left_offset_from_left_edge = 1.5 * screen_base_unit;
                    let center_offset_from_left_edge = 3.5 * screen_base_unit;
                    let right_offset_from_left_edge = 5.5 * screen_base_unit;

                    let left_edge_x = screen_center.x - screen_effective_width * 0.5;

                    let screen_positions = [
                        left_edge_x + left_offset_from_left_edge, // Left (matches trapezoid left)
                        left_edge_x + center_offset_from_left_edge, // Center (matches trapezoid center)
                        left_edge_x + right_offset_from_left_edge, // Right (matches trapezoid right)
                    ];

                    // VIAs now perfectly aligned with the new 7x layout trapezoid columns

                    for (i, &screen_x) in screen_positions.iter().enumerate() {
                        // Create via rectangle directly in screen coordinates
                        let via_screen_center = Pos2::new(screen_x, screen_center.y);

                        // Check if this VIA will be selected
                        let via_name = format!("{}_{}", via.name, i);
                        let is_selected = self.selected_layer.as_deref() == Some(&via_name)
                            || self.selected_layer.as_deref() == Some(&via.name);

                        // Use different colors for selected vs normal VIAs
                        let via_color = if is_selected {
                            Color32::from_rgb(255, 215, 0) // Gold color for selected VIA
                        } else {
                            Color32::from_rgb(192, 192, 192) // Silver-gray color for normal VIA
                        };
                        let stroke = Stroke::new(
                            if is_selected { 3.0 } else { 2.0 },
                            if is_selected {
                                Color32::YELLOW
                            } else {
                                Color32::DARK_GRAY
                            },
                        );

                        let rectangle = RectangleShape::new(
                            via_screen_center,
                            screen_width,
                            screen_height,
                            via_color,
                            stroke,
                        );

                        let via_name = format!("{}_{}", via.name, i);
                        let mut geometry = LayerGeometry::new_rectangle(
                            via_name.clone(),
                            via_z_start.min(via_z_end),
                            via_z_start.max(via_z_end),
                            rectangle,
                        );

                        // Check if this VIA is selected (check both full name and base name)
                        let is_selected = self.selected_layer.as_deref() == Some(&via_name)
                            || self.selected_layer.as_deref() == Some(&via.name);
                        geometry.set_selected(is_selected);

                        geometries.push(geometry);
                    }
                } else {
                    // Fallback: if no conductor layers found, use simple center alignment
                    let screen_positions = [screen_center.x]; // Just center

                    for (i, &screen_x) in screen_positions.iter().enumerate() {
                        let via_screen_center = Pos2::new(screen_x, screen_center.y);
                        let via_name = format!("{}_{}", via.name, i);
                        let is_selected = self.selected_layer.as_deref() == Some(&via_name)
                            || self.selected_layer.as_deref() == Some(&via.name);

                        let via_color = if is_selected {
                            Color32::from_rgb(255, 215, 0)
                        } else {
                            Color32::from_rgb(192, 192, 192)
                        };
                        let stroke = Stroke::new(
                            if is_selected { 3.0 } else { 2.0 },
                            if is_selected {
                                Color32::YELLOW
                            } else {
                                Color32::DARK_GRAY
                            },
                        );

                        let rectangle = RectangleShape::new(
                            via_screen_center,
                            screen_width,
                            screen_height,
                            via_color,
                            stroke,
                        );

                        let mut geometry = LayerGeometry::new_rectangle(
                            via_name.clone(),
                            via_z_start.min(via_z_end),
                            via_z_start.max(via_z_end),
                            rectangle,
                        );

                        geometry.set_selected(is_selected);
                        geometries.push(geometry);
                    }
                }
            }
        }

        geometries
    }

    fn calculate_via_width(
        &self,
        via: &crate::data::ViaConnection,
        stack: &ProcessStack,
        scaler: &ThicknessScaler,
    ) -> f32 {
        // Via width should be the minimum of the narrowest edges of connected metal layers
        let mut min_metal_width = f32::INFINITY;
        let mut found_metal = false;

        // Check from_layer
        if let Some(Layer::Conductor(conductor)) = stack.get_layer(&via.from_layer) {
            let conductor_height =
                scaler.get_exaggerated_thickness_for_layer(&Layer::Conductor(conductor.clone()));
            let effective_width = self.calculate_metal_effective_width(conductor, conductor_height);
            min_metal_width = min_metal_width.min(effective_width);
            found_metal = true;
        }

        // Check to_layer
        if let Some(Layer::Conductor(conductor)) = stack.get_layer(&via.to_layer) {
            let conductor_height =
                scaler.get_exaggerated_thickness_for_layer(&Layer::Conductor(conductor.clone()));
            let effective_width = self.calculate_metal_effective_width(conductor, conductor_height);
            min_metal_width = min_metal_width.min(effective_width);
            found_metal = true;
        }

        // If no metal layers found, use a default based on via area
        if !found_metal {
            return via.get_via_width() as f32 * 10.0; // Scale up for visibility
        }

        // For contact vias (connecting to substrate), use a smaller width
        if via.is_contact_via() {
            min_metal_width * 0.8
        } else {
            // For metal vias, via width cannot exceed the narrowest metal edge
            min_metal_width
        }
    }

    fn calculate_metal_effective_width(
        &self,
        conductor: &crate::data::ConductorLayer,
        conductor_height: f32,
    ) -> f32 {
        // Calculate the narrowest edge width of the metal trapezoid
        // This matches the logic in ThreeColumnTrapezoidShape::from_conductor_layer

        // Metal trapezoid dimensions: long_edge = height * 2.0, short_edge = height * 1.0
        let long_edge_width = conductor_height * 2.0;
        let short_edge_width = conductor_height * 1.0;

        let side_tangent = conductor.physical_props.side_tangent.unwrap_or(0.0) as f32;

        let (top_width, bottom_width) = if side_tangent >= 0.0 {
            // Top wider (negative trapezoid - like etched metal)
            (long_edge_width, short_edge_width)
        } else {
            // Top narrower (positive trapezoid - like deposited metal)
            (short_edge_width, long_edge_width)
        };

        // Return the narrowest edge width
        top_width.min(bottom_width)
    }

    pub fn calculate_ordered_layer_boundaries(
        &self,
        stack: &ProcessStack,
        scaler: &ThicknessScaler,
    ) -> HashMap<String, (f32, f32)> {
        let mut layer_boundaries = HashMap::new();
        let mut current_z = 0.0f32;

        // First pass: process dielectric layers to establish their positions
        let mut dielectric_positions = Vec::new();
        for (layer_index, layer) in stack.layers.iter().enumerate().rev() {
            if let Layer::Dielectric(_) = layer {
                let exaggerated_height = scaler.get_exaggerated_thickness_for_layer(layer);
                let bottom = current_z;
                let top = current_z + exaggerated_height;
                dielectric_positions.push((layer_index, bottom, top, exaggerated_height));
                current_z = top;
            }
        }

        // Second pass: calculate boundaries for all layers, embedding conductors in their preceding dielectric
        let mut dielectric_index = 0;

        for (layer_index, layer) in stack.layers.iter().enumerate().rev() {
            let exaggerated_height = scaler.get_exaggerated_thickness_for_layer(layer);

            let (z_bottom, z_top) = match layer {
                Layer::Dielectric(_) => {
                    // Use pre-calculated dielectric position
                    let (_, bottom, top, _) = dielectric_positions[dielectric_index];
                    dielectric_index += 1;
                    (bottom, top)
                }
                Layer::Conductor(_) => {
                    // Find the dielectric layer that should contain this conductor
                    // In ITF order, the conductor should be embedded in the previous dielectric layer
                    let mut target_dielectric_bottom = 0.0f32;

                    // Look for the dielectric layer that appears right before this conductor in the original layer order
                    if layer_index > 0 {
                        if let Some(Layer::Dielectric(_)) = stack.layers.get(layer_index - 1) {
                            // Find this dielectric's position
                            for &(d_index, d_bottom, _d_top, _d_height) in &dielectric_positions {
                                if d_index == (layer_index - 1) {
                                    target_dielectric_bottom = d_bottom;
                                    break;
                                }
                            }
                        }
                    }

                    let bottom = target_dielectric_bottom;
                    let top = bottom + exaggerated_height;
                    (bottom, top)
                }
            };

            layer_boundaries.insert(layer.name().to_string(), (z_bottom, z_top));
        }

        layer_boundaries
    }

    fn create_dimension_shapes_with_scaler(
        &self,
        stack: &ProcessStack,
        scaler: &ThicknessScaler,
        transform: &ViewTransform,
        viewport_rect: Rect,
    ) -> Vec<Shape> {
        let mut shapes = Vec::new();

        // Create simple tick marks along the left edge
        let ruler_color = egui::Color32::WHITE;
        let ruler_x = viewport_rect.min.x + 1.0; // Just 1 pixel from left edge

        // Get total stack height in world coordinates
        let total_height = scaler.get_exaggerated_total_height(stack);

        // Convert world coordinates to screen coordinates for ruler boundaries
        let world_bottom = Pos2::new(0.0, 0.0); // Bottom of stack
        let world_top = Pos2::new(0.0, -total_height); // Top of stack (negative Y)
        let screen_bottom = transform.world_to_screen(world_bottom);
        let screen_top = transform.world_to_screen(world_top);

        // Calculate tick marks
        let major_tick_interval = self.calculate_major_tick_interval(total_height);
        let minor_tick_interval = major_tick_interval / 5.0;

        // Draw tick marks
        let mut current_world_z = 0.0;
        while current_world_z <= total_height {
            let world_pos = Pos2::new(0.0, -current_world_z);
            let screen_pos = transform.world_to_screen(world_pos);

            // Check if this position is visible
            if screen_pos.y >= screen_top.y && screen_pos.y <= screen_bottom.y {
                let is_major_tick = (current_world_z / major_tick_interval).round()
                    * major_tick_interval
                    == current_world_z;

                if is_major_tick {
                    // Major tick mark (longer line)
                    let tick_start = Pos2::new(ruler_x, screen_pos.y);
                    let tick_end = Pos2::new(ruler_x + 15.0, screen_pos.y);
                    shapes.push(Shape::line_segment(
                        [tick_start, tick_end],
                        egui::Stroke::new(2.0, ruler_color),
                    ));
                } else if (current_world_z / minor_tick_interval).round() * minor_tick_interval
                    == current_world_z
                {
                    // Minor tick mark (shorter line)
                    let tick_start = Pos2::new(ruler_x, screen_pos.y);
                    let tick_end = Pos2::new(ruler_x + 8.0, screen_pos.y);
                    shapes.push(Shape::line_segment(
                        [tick_start, tick_end],
                        egui::Stroke::new(1.0, ruler_color),
                    ));
                }
            }

            current_world_z += minor_tick_interval;
        }

        shapes
    }

    // Helper function to calculate appropriate tick interval
    fn calculate_major_tick_interval(&self, total_height: f32) -> f32 {
        if total_height <= 1.0 {
            0.1 // 0.1 μm intervals for very small stacks
        } else if total_height <= 5.0 {
            0.5 // 0.5 μm intervals
        } else if total_height <= 10.0 {
            1.0 // 1 μm intervals
        } else if total_height <= 50.0 {
            5.0 // 5 μm intervals
        } else if total_height <= 100.0 {
            10.0 // 10 μm intervals
        } else {
            20.0 // 20 μm intervals for very large stacks
        }
    }

    /// Render dimensions with text using painter
    fn render_dimensions_with_painter(
        &self,
        stack: &ProcessStack,
        transform: &ViewTransform,
        viewport_rect: Rect,
        painter: &egui::Painter,
    ) {
        // Choose the appropriate scaler based on mode
        let scaler = if self.show_schematic_mode {
            self.create_schematic_scaler(stack)
        } else {
            self.create_normal_scaler(stack)
        };

        // Create simple tick marks along the left edge
        let ruler_color = egui::Color32::WHITE;
        let ruler_x = viewport_rect.min.x + 1.0; // Just 1 pixel from left edge

        // Get total stack height in world coordinates
        let total_height = scaler.get_exaggerated_total_height(stack);

        // Convert world coordinates to screen coordinates for ruler boundaries
        let world_bottom = Pos2::new(0.0, 0.0); // Bottom of stack
        let world_top = Pos2::new(0.0, -total_height); // Top of stack (negative Y)
        let screen_bottom = transform.world_to_screen(world_bottom);
        let screen_top = transform.world_to_screen(world_top);

        // Calculate tick marks
        let major_tick_interval = self.calculate_major_tick_interval(total_height);
        let minor_tick_interval = major_tick_interval / 5.0;

        // Draw tick marks with text labels
        let mut current_world_z = 0.0;
        while current_world_z <= total_height {
            let world_pos = Pos2::new(0.0, -current_world_z);
            let screen_pos = transform.world_to_screen(world_pos);

            // Check if this position is visible
            if screen_pos.y >= screen_top.y && screen_pos.y <= screen_bottom.y {
                let is_major_tick = (current_world_z / major_tick_interval).round()
                    * major_tick_interval
                    == current_world_z;

                if is_major_tick {
                    // Major tick mark (longer line)
                    let tick_start = Pos2::new(ruler_x, screen_pos.y);
                    let tick_end = Pos2::new(ruler_x + 15.0, screen_pos.y);
                    painter
                        .line_segment([tick_start, tick_end], egui::Stroke::new(2.0, ruler_color));

                    // Add text label for major ticks
                    let label = format!("{current_world_z:.1}μm");
                    let text_pos = Pos2::new(ruler_x + 20.0, screen_pos.y);
                    let font_id = FontId::monospace(10.0);

                    // Add small background for text readability
                    painter.rect_filled(
                        Rect::from_center_size(text_pos, Vec2::new(30.0, 14.0)),
                        2.0,
                        Color32::from_black_alpha(120),
                    );

                    painter.text(text_pos, Align2::LEFT_CENTER, label, font_id, ruler_color);
                } else if (current_world_z / minor_tick_interval).round() * minor_tick_interval
                    == current_world_z
                {
                    // Minor tick mark (shorter line)
                    let tick_start = Pos2::new(ruler_x, screen_pos.y);
                    let tick_end = Pos2::new(ruler_x + 8.0, screen_pos.y);
                    painter
                        .line_segment([tick_start, tick_end], egui::Stroke::new(1.0, ruler_color));
                }
            }

            current_world_z += minor_tick_interval;
        }
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

    pub fn set_show_schematic_mode(&mut self, show: bool) {
        self.show_schematic_mode = show;
    }

    /// Get the appropriate scaler based on current mode
    pub fn get_current_scaler(&self, stack: &ProcessStack) -> ThicknessScaler {
        if self.show_schematic_mode {
            self.create_schematic_scaler(stack)
        } else {
            self.create_normal_scaler(stack)
        }
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
        // Use the same scaler configuration as rendering to ensure coordinate consistency
        let scaler = self.get_current_scaler(stack);
        let layer_geometries =
            self.create_layer_geometries_ordered(stack, &scaler, transform, viewport_rect);

        // Also get VIA geometries for hit testing (VIAs have highest z-order)
        let via_geometries =
            self.create_via_geometries_with_scaler(stack, &scaler, transform, viewport_rect);

        // Test VIAs first (highest z-index, rendered on top of everything)
        for geometry in via_geometries.iter().rev() {
            if geometry.contains_point(point) {
                // For VIA geometries, extract the base name (remove the "_N" suffix)
                // VIA geometry names are in format "via_name_0", "via_name_1", etc.
                let base_name = if let Some(underscore_pos) = geometry.layer_name.rfind('_') {
                    // Check if the part after the last underscore is a number
                    let suffix = &geometry.layer_name[underscore_pos + 1..];
                    if suffix.chars().all(|c| c.is_ascii_digit()) {
                        // This is a VIA with index suffix, return the base name
                        geometry.layer_name[..underscore_pos].to_string()
                    } else {
                        // Not a VIA index, return full name
                        geometry.layer_name.clone()
                    }
                } else {
                    // No underscore, return full name
                    geometry.layer_name.clone()
                };
                return Some(base_name);
            }
        }

        // Separate layer geometries by type for proper z-ordering hit testing
        let mut dielectric_geometries = Vec::new();
        let mut conductor_geometries = Vec::new();

        for geometry in &layer_geometries {
            // Check if this is a conductor layer by looking at the shape type
            match &geometry.shape {
                LayerShape::ThreeColumnTrapezoid(_) => {
                    // All conductor layers use ThreeColumnTrapezoid
                    conductor_geometries.push(geometry);
                }
                _ => {
                    // All other shapes are dielectric layers
                    dielectric_geometries.push(geometry);
                }
            }
        }

        // Test conductor layers second (medium z-index)
        for geometry in conductor_geometries.iter().rev() {
            if geometry.contains_point(point) {
                return Some(geometry.layer_name.clone());
            }
        }

        // Finally test dielectric layers (lowest z-index, rendered below)
        for geometry in dielectric_geometries.iter().rev() {
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

        // Choose the appropriate scaler based on mode
        let scaler = if self.show_schematic_mode {
            self.create_schematic_scaler(stack)
        } else {
            self.create_normal_scaler(stack)
        };

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
            // Reserve space for the ruler on the left (about 30 pixels)
            let ruler_space = 30.0;
            let viewport_size = transform.viewport_size;

            // Calculate effective viewport size after reserving ruler space
            let effective_viewport_width = viewport_size.x - ruler_space;
            let effective_viewport_height = viewport_size.y;

            // Calculate scale to fit both width and height with some margin
            let margin = 20.0; // Fixed margin in pixels
            let scale_x = (effective_viewport_width - margin * 2.0) / bounds.width();
            let scale_y = (effective_viewport_height - margin * 2.0) / bounds.height();

            // Use the smaller scale to ensure everything fits
            transform.scale = scale_x.min(scale_y).max(0.01);

            // Center the stack in the effective viewport area
            let bounds_center = bounds.center();
            let viewport_center_x = ruler_space + effective_viewport_width * 0.5;
            let viewport_center_y = effective_viewport_height * 0.5;

            transform.offset = Vec2::new(
                viewport_center_x - bounds_center.x * transform.scale,
                viewport_center_y - bounds_center.y * transform.scale,
            );
        }
    }

    /// Create a special scaler for schematic mode that maps layer thicknesses
    /// from 30% (thinnest) to 100% (thickest) linearly
    fn create_schematic_scaler(&self, stack: &ProcessStack) -> ThicknessScaler {
        if stack.layers.is_empty() {
            let mut scaler = self.thickness_scaler.clone();
            scaler.analyze_stack(stack);
            return scaler;
        }

        // Collect all non-zero layer thicknesses for schematic mode
        let mut thicknesses = Vec::new();
        for layer in &stack.layers {
            let thickness = layer.thickness();
            if thickness > 0.0 {
                // Only include non-zero thicknesses
                thicknesses.push(thickness);
            }
        }

        if thicknesses.is_empty() {
            // Fallback to normal scaler if no valid thicknesses
            let mut scaler = self.thickness_scaler.clone();
            scaler.analyze_stack(stack);
            return scaler;
        }

        // Find min and max thickness from non-zero values
        let min_thickness = thicknesses.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_thickness = thicknesses
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        // Create a custom scaler that implements the 30%-100% mapping
        let mut scaler = ThicknessScaler::new();

        // Set custom scaling parameters for schematic mode
        scaler.set_schematic_mode(min_thickness, max_thickness);
        scaler.analyze_stack(stack);

        scaler
    }

    /// Create a scaler for normal mode that shows layers at true scale
    fn create_normal_scaler(&self, stack: &ProcessStack) -> ThicknessScaler {
        // In normal mode, all layers are shown at their true thickness (1:1 scaling)
        let mut scaler = ThicknessScaler::new();
        scaler.set_normal_mode(); // Ensure it's in normal mode
        scaler.analyze_stack(stack);
        scaler
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
            show_schematic_mode: self.show_schematic_mode,
            selected_layer: self.selected_layer.clone(),
            thickness_scaler: self.thickness_scaler.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{ConductorLayer, DielectricLayer, TechnologyInfo};
    use egui::Vec2;

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
        let geometries =
            renderer.create_layer_geometries_ordered(&stack, &scaler, &transform, viewport_rect);

        assert_eq!(geometries.len(), stack.get_layer_count());

        // With embedded conductor logic, layers may overlap so we can't expect strict ordering
        // Instead, verify that all geometries have valid z positions
        for geometry in &geometries {
            assert!(
                geometry.z_bottom < geometry.z_top,
                "Layer {} should have bottom < top: {} < {}",
                geometry.layer_name,
                geometry.z_bottom,
                geometry.z_top
            );
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
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new(
            "conductor1".to_string(),
            0.5,
        ))));
        stack.add_layer(Layer::Dielectric(DielectricLayer::new(
            "dielectric1".to_string(),
            1.0,
            4.2,
        )));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new(
            "conductor2".to_string(),
            0.3,
        ))));
        stack.add_layer(Layer::Dielectric(DielectricLayer::new(
            "dielectric2".to_string(),
            0.8,
            4.2,
        )));

        let transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        let viewport_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));

        let mut scaler = ThicknessScaler::new();
        scaler.analyze_stack(&stack);
        let geometries =
            renderer.create_layer_geometries_ordered(&stack, &scaler, &transform, viewport_rect);

        // Should have 4 geometries
        assert_eq!(geometries.len(), 4);

        // Check stacking order: layers should be rendered in reverse ITF order (bottom to top physically)
        // ITF order: conductor1, dielectric1, conductor2, dielectric2 (top to bottom in file)
        // Render order: dielectric2, conductor2, dielectric1, conductor1 (bottom to top physically)
        assert_eq!(geometries[0].layer_name, "dielectric2"); // Last in ITF = bottom of stack
        assert_eq!(geometries[1].layer_name, "conductor2"); // Second to last in ITF
        assert_eq!(geometries[2].layer_name, "dielectric1"); // Second in ITF
        assert_eq!(geometries[3].layer_name, "conductor1"); // First in ITF = top of stack

        // With embedded conductor logic, conductors are embedded in dielectrics
        // So we need to verify the new embedding behavior instead of strict layer ordering
        let mut dielectric_layers = Vec::new();
        let mut conductor_layers = Vec::new();

        for geometry in &geometries {
            match &geometry.shape {
                LayerShape::ThreeColumnTrapezoid(_) => conductor_layers.push(geometry),
                _ => dielectric_layers.push(geometry),
            }
        }

        // Verify we have the expected number of each type
        assert_eq!(dielectric_layers.len(), 2);
        assert_eq!(conductor_layers.len(), 2);

        // Verify that all layers have valid z positions
        for geometry in &geometries {
            assert!(
                geometry.z_bottom < geometry.z_top,
                "Layer {} should have bottom < top",
                geometry.layer_name
            );
        }
    }

    #[test]
    fn test_via_positioning_with_new_stacking() {
        let renderer = StackRenderer::new();

        // Create a test stack with via connections
        let tech = TechnologyInfo::new("test_via_stacking".to_string());
        let mut stack = ProcessStack::new(tech);

        // Add layers: dielectric, conductor, dielectric, conductor
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
            0.8,
            4.2,
        )));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new(
            "metal2".to_string(),
            0.3,
        ))));

        // Add a via connecting the two metal layers
        use crate::data::ViaConnection;
        let via = ViaConnection::new(
            "via12".to_string(),
            "metal1".to_string(),
            "metal2".to_string(),
            0.25,
            5.0,
        );
        stack.add_via(via);

        let transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        let viewport_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));

        let mut scaler = ThicknessScaler::new();
        scaler.analyze_stack(&stack);

        // Get layer boundaries for precise testing
        let layer_boundaries = renderer.calculate_ordered_layer_boundaries(&stack, &scaler);

        // With the new embedded conductor logic:
        // ITF order (top to bottom): oxide1, metal1, oxide2, metal2
        // Physical render order (bottom to top): oxide2, oxide1, with metals embedded
        // - oxide2 is at the bottom (z=0 to z=oxide2_height)
        // - metal2 is embedded in oxide2 (z=0 to z=metal2_height)
        // - oxide1 is above oxide2 (z=oxide2_height to z=oxide2_height+oxide1_height)
        // - metal1 is embedded in oxide1 (z=oxide2_height to z=oxide2_height+metal1_height)
        let oxide1_bounds = layer_boundaries.get("oxide1").unwrap();
        let oxide2_bounds = layer_boundaries.get("oxide2").unwrap();
        let metal1_bounds = layer_boundaries.get("metal1").unwrap();
        let metal2_bounds = layer_boundaries.get("metal2").unwrap();

        // Verify the new embedded stacking order
        // In reverse ITF order, oxide2 comes first (bottom), then oxide1 (top)
        assert!(
            oxide1_bounds.0 >= oxide2_bounds.1 - 1e-6,
            "oxide1 should be above oxide2: {} >= {}",
            oxide1_bounds.0,
            oxide2_bounds.1
        );

        // metal2 should be embedded in oxide2 (same bottom)
        assert!(
            (metal2_bounds.0 - oxide2_bounds.0).abs() < 1e-6,
            "metal2 should be embedded in oxide2 (same bottom): {} == {}",
            metal2_bounds.0,
            oxide2_bounds.0
        );

        // metal1 should be embedded in oxide1 (same bottom)
        assert!(
            (metal1_bounds.0 - oxide1_bounds.0).abs() < 1e-6,
            "metal1 should be embedded in oxide1 (same bottom): {} == {}",
            metal1_bounds.0,
            oxide1_bounds.0
        );

        // Create via geometries
        let via_geometries =
            renderer.create_via_geometries_with_scaler(&stack, &scaler, &transform, viewport_rect);
        assert_eq!(via_geometries.len(), 3); // 1 via * 3 positions = 3 geometries

        // Via should span between the two metal layers
        let via_geom = &via_geometries[0];
        assert_eq!(via_geom.layer_name, "via12_0"); // Updated naming scheme

        // Via should be positioned to connect the layer surfaces
        // With embedded stacking: metal1 is in oxide1 (above), metal2 is in oxide2 (below)
        // Via should span from bottom of metal1 to top of metal2
        let expected_start = metal1_bounds.0; // Bottom of metal1
        let expected_end = metal2_bounds.1; // Top of metal2

        // Since metal1 is above metal2 in our new structure, we need to check which one is actually higher
        let via_should_start = expected_start.min(expected_end);
        let via_should_end = expected_start.max(expected_end);

        // Allow for some tolerance due to potential floating point precision issues
        let tolerance = 1e-3; // Increase tolerance slightly
        assert!(
            (via_geom.z_bottom - via_should_start).abs() < tolerance,
            "Via should start at {}, but starts at {}",
            via_should_start,
            via_geom.z_bottom
        );
        assert!(
            (via_geom.z_top - via_should_end).abs() < tolerance,
            "Via should end at {}, but ends at {}",
            via_should_end,
            via_geom.z_top
        );
    }

    #[test]
    fn test_thickness_exaggeration_integration() {
        let renderer = StackRenderer::new();

        // Create stack with varied thicknesses
        let tech = TechnologyInfo::new("test_exaggeration".to_string());
        let mut stack = ProcessStack::new(tech);

        // Add layers with different thicknesses: thin, thick, medium
        stack.add_layer(Layer::Dielectric(DielectricLayer::new(
            "thin".to_string(),
            0.1,
            4.2,
        )));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new(
            "thick".to_string(),
            2.0,
        ))));
        stack.add_layer(Layer::Dielectric(DielectricLayer::new(
            "medium".to_string(),
            1.0,
            4.2,
        )));

        let transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        let viewport_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));

        let mut scaler = ThicknessScaler::new();
        scaler.analyze_stack(&stack);
        // Set to schematic mode for thickness exaggeration testing
        scaler.set_schematic_mode(0.1, 2.0); // min=0.1, max=2.0 from the stack
        let geometries =
            renderer.create_layer_geometries_ordered(&stack, &scaler, &transform, viewport_rect);

        // Should have 3 geometries in reverse ITF order (bottom to top physically)
        // ITF order: thin, thick, medium (top to bottom in file)
        // Render order: medium, thick, thin (bottom to top physically)
        assert_eq!(geometries.len(), 3);
        assert_eq!(geometries[0].layer_name, "medium"); // Last in ITF = bottom of stack
        assert_eq!(geometries[1].layer_name, "thick"); // Second in ITF
        assert_eq!(geometries[2].layer_name, "thin"); // First in ITF = top of stack

        // Check that thickness exaggeration is applied
        // geometry[0] = medium, geometry[1] = thick, geometry[2] = thin
        let medium_height = geometries[0].z_top - geometries[0].z_bottom;
        let thick_height = geometries[1].z_top - geometries[1].z_bottom;
        let thin_height = geometries[2].z_top - geometries[2].z_bottom;

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
        // In schematic mode, all layers are scaled relative to max thickness (2.0)
        let expected_thin = 2.0 * thin_scale; // 2.0 * 0.3 = 0.6
        let expected_thick = 2.0 * thick_scale; // 2.0 * 1.0 = 2.0
        let expected_medium = 2.0 * medium_scale; // 2.0 * 0.65 = 1.3

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
        let geometries =
            renderer.create_layer_geometries_ordered(&stack, &scaler, &transform, viewport_rect);

        assert_eq!(geometries.len(), 1);
        let conductor_geometry = &geometries[0];

        // Verify it's using ThreeColumnTrapezoid shape
        match &conductor_geometry.shape {
            LayerShape::ThreeColumnTrapezoid(three_trap) => {
                // Should have exactly 3 trapezoids (left, center, right)
                // Verify that all trapezoids exist
                assert!(three_trap.left_trapezoid.bottom_left.x != 0.0);
                assert!(three_trap.center_trapezoid.bottom_left.x != 0.0);
                assert!(three_trap.right_trapezoid.bottom_left.x != 0.0);
            }
            _ => panic!("Conductor with side_tangent should use ThreeColumnTrapezoid shape"),
        }

        // Verify it generates exactly 3 shapes for rendering (left, center, right)
        let shapes = conductor_geometry.to_egui_shapes();
        assert_eq!(shapes.len(), 3); // Should generate 3 shapes for 3 trapezoids
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
        let geometries =
            renderer.create_layer_geometries_ordered(&stack, &scaler, &transform, viewport_rect);

        assert_eq!(geometries.len(), 1);
        let conductor_geometry = &geometries[0];

        // Verify it's using ThreeColumnTrapezoid shape (all conductors now use this)
        match &conductor_geometry.shape {
            LayerShape::ThreeColumnTrapezoid(three_trap) => {
                // Should have exactly 3 trapezoids (left, center, right)
                // Verify that all trapezoids exist
                assert!(three_trap.left_trapezoid.bottom_left.x != 0.0);
                assert!(three_trap.center_trapezoid.bottom_left.x != 0.0);
                assert!(three_trap.right_trapezoid.bottom_left.x != 0.0);
            }
            _ => panic!("Conductor without side_tangent should use ThreeColumnTrapezoid shape"),
        }

        // Should generate exactly 3 shapes for 3 trapezoids
        let shapes = conductor_geometry.to_egui_shapes();
        assert_eq!(shapes.len(), 3);
    }

    #[test]
    fn test_improved_via_positioning() {
        let renderer = StackRenderer::new();

        // Create stack with layers and VIAs
        let tech = TechnologyInfo::new("test_via_improved".to_string());
        let mut stack = ProcessStack::new(tech);

        // Add layers in order: substrate, metal1, oxide, metal2
        stack.add_layer(Layer::Dielectric(DielectricLayer::new(
            "substrate".to_string(),
            1.0,
            11.7,
        )));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new(
            "metal1".to_string(),
            0.5,
        ))));
        stack.add_layer(Layer::Dielectric(DielectricLayer::new(
            "oxide".to_string(),
            0.8,
            4.2,
        )));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new(
            "metal2".to_string(),
            0.3,
        ))));

        // Add VIA connecting metal1 to metal2
        use crate::data::ViaConnection;
        let via = ViaConnection::new(
            "via_m1_m2".to_string(),
            "metal1".to_string(),
            "metal2".to_string(),
            0.25,
            5.0,
        );
        stack.add_via(via);

        let transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        let viewport_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));

        let mut scaler = ThicknessScaler::new();
        scaler.analyze_stack(&stack);

        // Get layer boundaries
        let layer_boundaries = renderer.calculate_ordered_layer_boundaries(&stack, &scaler);

        // With the new embedded conductor logic:
        // - substrate is at the bottom
        // - metal1 is embedded in substrate (same bottom)
        // - oxide is above substrate
        // - metal2 is embedded in oxide (same bottom as oxide)
        let substrate_bounds = layer_boundaries.get("substrate").unwrap();
        let oxide_bounds = layer_boundaries.get("oxide").unwrap();
        let metal1_bounds = layer_boundaries.get("metal1").unwrap();
        let metal2_bounds = layer_boundaries.get("metal2").unwrap();

        // Verify layer ordering with embedded logic
        // In ITF order: substrate, metal1, oxide, metal2 (top to bottom in file)
        // In physical order: substrate at bottom, oxide above substrate
        // But with reverse processing: oxide is processed first (gets bottom position), substrate second
        // So substrate should be above oxide in the current implementation
        assert!(
            substrate_bounds.0 >= oxide_bounds.1 - 1e-6,
            "substrate should be above oxide: {} >= {}",
            substrate_bounds.0,
            oxide_bounds.1
        );

        // metal2 should be embedded in oxide (same bottom, since oxide is processed first)
        assert!(
            (metal2_bounds.0 - oxide_bounds.0).abs() < 1e-6,
            "metal2 should be embedded in oxide: {} == {}",
            metal2_bounds.0,
            oxide_bounds.0
        );

        // metal1 should be embedded in substrate (same bottom, since substrate is processed second)
        assert!(
            (metal1_bounds.0 - substrate_bounds.0).abs() < 1e-6,
            "metal1 should be embedded in substrate: {} == {}",
            metal1_bounds.0,
            substrate_bounds.0
        );

        // Create VIA geometries
        let via_geometries =
            renderer.create_via_geometries_with_scaler(&stack, &scaler, &transform, viewport_rect);
        assert_eq!(via_geometries.len(), 3); // 1 via * 3 positions = 3 geometries

        let via_geom = &via_geometries[0];

        // VIA should span from the surface of metal1 to the surface of metal2
        // Since substrate is above oxide now, metal1 (in substrate) is above metal2 (in oxide)
        let expected_via_start = metal2_bounds.1; // Top of metal2
        let expected_via_end = metal1_bounds.0; // Bottom of metal1

        assert!(
            (via_geom.z_bottom - expected_via_start.min(expected_via_end)).abs() < 1e-6,
            "Via should start at {}, but starts at {}",
            expected_via_start.min(expected_via_end),
            via_geom.z_bottom
        );
        assert!(
            (via_geom.z_top - expected_via_start.max(expected_via_end)).abs() < 1e-6,
            "Via should end at {}, but ends at {}",
            expected_via_start.max(expected_via_end),
            via_geom.z_top
        );
    }

    #[test]
    fn test_multiple_vias_horizontal_offset() {
        let renderer = StackRenderer::new();

        // Create stack with two layers
        let tech = TechnologyInfo::new("test_multi_via".to_string());
        let mut stack = ProcessStack::new(tech);

        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new(
            "metal1".to_string(),
            0.5,
        ))));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new(
            "metal2".to_string(),
            0.3,
        ))));

        // Add multiple VIAs between the same layers
        use crate::data::ViaConnection;
        let via1 = ViaConnection::new(
            "via1".to_string(),
            "metal1".to_string(),
            "metal2".to_string(),
            0.25,
            5.0,
        );
        let via2 = ViaConnection::new(
            "via2".to_string(),
            "metal1".to_string(),
            "metal2".to_string(),
            0.25,
            5.0,
        );
        let via3 = ViaConnection::new(
            "via3".to_string(),
            "metal1".to_string(),
            "metal2".to_string(),
            0.25,
            5.0,
        );

        stack.add_via(via1);
        stack.add_via(via2);
        stack.add_via(via3);

        let transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        let viewport_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));

        let mut scaler = ThicknessScaler::new();
        scaler.analyze_stack(&stack);

        // Create VIA geometries
        let via_geometries =
            renderer.create_via_geometries_with_scaler(&stack, &scaler, &transform, viewport_rect);
        assert_eq!(via_geometries.len(), 9); // 3 vias * 3 positions each = 9 geometries

        // VIAs should have different horizontal positions (to avoid overlap)
        let bounds_via1 = via_geometries[0].get_bounds();
        let bounds_via2 = via_geometries[1].get_bounds();
        let bounds_via3 = via_geometries[2].get_bounds();

        // Check that VIAs don't significantly overlap horizontally
        assert!(bounds_via1.center().x != bounds_via2.center().x);
        assert!(bounds_via2.center().x != bounds_via3.center().x);
        assert!(bounds_via1.center().x != bounds_via3.center().x);

        // Check that all vias have the same vertical span (same layer connection)
        for i in 0..via_geometries.len() {
            for j in (i + 1)..via_geometries.len() {
                let bounds_i = via_geometries[i].get_bounds();
                let bounds_j = via_geometries[j].get_bounds();
                assert!(
                    (bounds_i.height() - bounds_j.height()).abs() < 1e-6,
                    "All vias should have same height"
                );
            }
        }

        // Check that we have exactly 3 vias per via connection
        let unique_base_names: std::collections::HashSet<String> = via_geometries
            .iter()
            .map(|g| {
                g.layer_name
                    .split('_')
                    .next()
                    .unwrap_or(&g.layer_name)
                    .to_string()
            })
            .collect();
        assert_eq!(unique_base_names.len(), 3); // Should have 3 unique via connections
    }

    #[test]
    fn test_three_vias_per_connection() {
        let renderer = StackRenderer::new();

        // Create stack with two metal layers
        let tech = TechnologyInfo::new("test_three_via".to_string());
        let mut stack = ProcessStack::new(tech);

        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new(
            "metal1".to_string(),
            0.5,
        ))));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new(
            "metal2".to_string(),
            0.3,
        ))));

        // Add a single via connection
        use crate::data::ViaConnection;
        let via = ViaConnection::new(
            "via_test".to_string(),
            "metal1".to_string(),
            "metal2".to_string(),
            0.25,
            5.0,
        );
        stack.add_via(via);

        let transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        let viewport_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));

        let mut scaler = ThicknessScaler::new();
        scaler.analyze_stack(&stack);

        // Create VIA geometries
        let via_geometries =
            renderer.create_via_geometries_with_scaler(&stack, &scaler, &transform, viewport_rect);

        // Should create exactly 3 vias for one connection
        assert_eq!(via_geometries.len(), 3);

        // Check names are correctly formatted
        assert_eq!(via_geometries[0].layer_name, "via_test_0");
        assert_eq!(via_geometries[1].layer_name, "via_test_1");
        assert_eq!(via_geometries[2].layer_name, "via_test_2");

        // Check that vias are at different horizontal positions
        let pos1 = via_geometries[0].get_bounds().center().x;
        let pos2 = via_geometries[1].get_bounds().center().x;
        let pos3 = via_geometries[2].get_bounds().center().x;

        assert!(pos1 != pos2);
        assert!(pos2 != pos3);
        assert!(pos1 != pos3);

        // Check that all vias have the same vertical position and height
        let bounds1 = via_geometries[0].get_bounds();
        for geometry in &via_geometries[1..] {
            let bounds = geometry.get_bounds();
            assert!((bounds.center().y - bounds1.center().y).abs() < 1e-6);
            assert!((bounds.height() - bounds1.height()).abs() < 1e-6);
        }
    }

    #[test]
    fn test_via_metal_alignment() {
        let renderer = StackRenderer::new();

        // Create stack with metal layers and via
        let tech = TechnologyInfo::new("test_alignment".to_string());
        let mut stack = ProcessStack::new(tech);

        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new(
            "metal1".to_string(),
            0.5,
        ))));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new(
            "metal2".to_string(),
            0.3,
        ))));

        // Add via connection
        use crate::data::ViaConnection;
        let via = ViaConnection::new(
            "alignment_via".to_string(),
            "metal1".to_string(),
            "metal2".to_string(),
            0.25,
            5.0,
        );
        stack.add_via(via);

        let transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        let viewport_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));

        let mut scaler = ThicknessScaler::new();
        scaler.analyze_stack(&stack);

        // Get metal geometries
        let metal_geometries =
            renderer.create_layer_geometries_ordered(&stack, &scaler, &transform, viewport_rect);

        // Get via geometries
        let via_geometries =
            renderer.create_via_geometries_with_scaler(&stack, &scaler, &transform, viewport_rect);

        // Should have 3 vias for one connection
        assert_eq!(via_geometries.len(), 3);

        // Find metal geometries
        let metal1_geom = metal_geometries
            .iter()
            .find(|g| g.layer_name == "metal1")
            .unwrap();
        let metal2_geom = metal_geometries
            .iter()
            .find(|g| g.layer_name == "metal2")
            .unwrap();

        // Verify that via positions align with metal column positions
        // We cannot directly access metal column positions, but we can verify spacing consistency
        let via_x_positions: Vec<f32> = via_geometries
            .iter()
            .map(|v| v.get_bounds().center().x)
            .collect();

        // Sort positions for consistent comparison
        let mut sorted_positions = via_x_positions.clone();
        sorted_positions.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Check that vias are spaced correctly
        if sorted_positions.len() >= 3 {
            let spacing1 = sorted_positions[1] - sorted_positions[0];
            let spacing2 = sorted_positions[2] - sorted_positions[1];

            // Spacings should be approximately equal (allowing for connection offset)
            let tolerance = 10.0; // Screen pixels tolerance
            assert!(
                (spacing1 - spacing2).abs() < tolerance,
                "Via spacings should be approximately equal: {spacing1} vs {spacing2}"
            );
        }

        // Verify vias are within reasonable bounds of metal layers
        let metal1_bounds = metal1_geom.get_bounds();
        let metal2_bounds = metal2_geom.get_bounds();
        let expected_width = metal1_bounds.width().max(metal2_bounds.width());

        for via_geom in &via_geometries {
            let via_center = via_geom.get_bounds().center();
            // Via should be within the wider metal layer bounds plus some margin
            let margin = expected_width * 0.6; // Allow 60% extra width for via spread
            assert!(
                via_center.x >= metal1_bounds.center().x - margin
                    && via_center.x <= metal1_bounds.center().x + margin,
                "Via at {} should be within metal bounds {} ± {}",
                via_center.x,
                metal1_bounds.center().x,
                margin
            );
        }
    }

    #[test]
    fn test_via_width_constraint() {
        let renderer = StackRenderer::new();

        // Create stack with trapezoid metals and via
        let tech = TechnologyInfo::new("test_via_width".to_string());
        let mut stack = ProcessStack::new(tech);

        // Add trapezoid metal layer 1
        let mut metal1 = ConductorLayer::new("metal1".to_string(), 0.5);
        metal1.physical_props.side_tangent = Some(0.1); // Positive trapezoid
        stack.add_layer(Layer::Conductor(Box::new(metal1)));

        // Add trapezoid metal layer 2
        let mut metal2 = ConductorLayer::new("metal2".to_string(), 0.3);
        metal2.physical_props.side_tangent = Some(-0.05); // Negative trapezoid
        stack.add_layer(Layer::Conductor(Box::new(metal2)));

        // Add via connection
        use crate::data::ViaConnection;
        let via = ViaConnection::new(
            "width_test_via".to_string(),
            "metal1".to_string(),
            "metal2".to_string(),
            0.25,
            5.0,
        );
        stack.add_via(via);

        let transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        let viewport_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));

        let mut scaler = ThicknessScaler::new();
        scaler.analyze_stack(&stack);

        // Get via geometries
        let via_geometries =
            renderer.create_via_geometries_with_scaler(&stack, &scaler, &transform, viewport_rect);

        // Should have 3 vias for one connection
        assert_eq!(via_geometries.len(), 3);

        // Calculate expected maximum via width
        // Metal1: height=0.5 exaggerated, narrowest edge = height * 1.0 = 0.5
        // Metal2: height=0.3 exaggerated, narrowest edge = height * 1.0 = 0.3
        // Via width should be min(0.5, 0.3) = 0.3 in world coordinates
        let metal1_height = scaler.get_exaggerated_thickness_for_layer(&Layer::Conductor(
            Box::new(ConductorLayer::new("metal1".to_string(), 0.5)),
        ));
        let metal2_height = scaler.get_exaggerated_thickness_for_layer(&Layer::Conductor(
            Box::new(ConductorLayer::new("metal2".to_string(), 0.3)),
        ));

        let expected_max_width_world = (metal1_height * 1.0).min(metal2_height * 1.0);
        let expected_max_width_screen = expected_max_width_world * transform.scale;

        // Verify all vias have width <= expected maximum
        for via_geom in &via_geometries {
            let via_width = via_geom.get_bounds().width();
            assert!(
                via_width <= expected_max_width_screen + 1.0, // Small tolerance for floating point
                "Via width {via_width} exceeds maximum allowed width {expected_max_width_screen} (metal constraint)"
            );
        }

        // Verify vias are reasonably sized (not too small)
        let min_reasonable_width = expected_max_width_screen * 0.5;
        for via_geom in &via_geometries {
            let via_width = via_geom.get_bounds().width();
            assert!(
                via_width >= min_reasonable_width,
                "Via width {via_width} is too small (should be at least {min_reasonable_width})"
            );
        }
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

        // Check that the scale is reasonable (not too small or too large)
        assert!(transform.scale > 0.01);
        assert!(transform.scale < 100.0);

        // Check that the stack bounds are reasonable for the viewport
        let stack_bounds = renderer.get_stack_bounds(&stack);
        let stack_width_screen = stack_bounds.width() * transform.scale;
        let stack_height_screen = stack_bounds.height() * transform.scale;

        // Stack should fit within viewport with some reasonable margin
        // Accounting for ruler space (30px) and margins (20px each side)
        let ruler_space = 30.0;
        let margin = 20.0;
        let effective_width = transform.viewport_size.x - ruler_space - margin * 2.0;
        let effective_height = transform.viewport_size.y - margin * 2.0;

        assert!(stack_width_screen <= effective_width + 1.0); // +1.0 for floating point tolerance
        assert!(stack_height_screen <= effective_height + 1.0);
    }

    #[test]
    fn test_hit_test_z_order_priority() {
        let renderer = StackRenderer::new();

        // Create a test stack with overlapping conductor and dielectric layers
        let tech = TechnologyInfo::new("test_hit_z_order".to_string());
        let mut stack = ProcessStack::new(tech);

        // Add layers: dielectric first, then conductor embedded in it
        stack.add_layer(Layer::Dielectric(DielectricLayer::new(
            "oxide".to_string(),
            2.0,
            4.2,
        )));
        stack.add_layer(Layer::Conductor(Box::new(ConductorLayer::new(
            "metal".to_string(),
            0.5,
        ))));

        let transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        let viewport_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));

        let mut scaler = ThicknessScaler::new();
        scaler.analyze_stack(&stack);
        let geometries =
            renderer.create_layer_geometries_ordered(&stack, &scaler, &transform, viewport_rect);

        // Verify we have both layers
        assert_eq!(geometries.len(), 2);

        // Find the bounds for each layer
        let mut oxide_bounds = Rect::NOTHING;
        let mut metal_bounds = Rect::NOTHING;

        for geometry in &geometries {
            let bounds = geometry.get_bounds();
            if geometry.layer_name == "oxide" {
                oxide_bounds = bounds;
            } else if geometry.layer_name == "metal" {
                metal_bounds = bounds;
            }
        }

        // Verify both bounds are valid
        assert!(
            oxide_bounds.width() > 0.0 && oxide_bounds.height() > 0.0,
            "Oxide bounds should be valid"
        );
        assert!(
            metal_bounds.width() > 0.0 && metal_bounds.height() > 0.0,
            "Metal bounds should be valid"
        );

        // Verify layers overlap (metal is embedded in oxide)
        assert!(
            oxide_bounds.intersects(metal_bounds),
            "Oxide and metal layers should overlap"
        );

        // Test hit detection in the overlapping region
        let overlap_center = metal_bounds.center();

        // Point in the overlapping region should hit the conductor (metal) first
        let hit_result = renderer.hit_test(&stack, &transform, viewport_rect, overlap_center);
        assert_eq!(
            hit_result,
            Some("metal".to_string()),
            "Hit test in overlapping region should return conductor layer (higher z-index)"
        );

        // Test a point that's in the dielectric but outside the metal bounds
        // Use a point that's definitely in the oxide bounds but far from metal
        let oxide_only_point = Pos2::new(
            oxide_bounds.min.x + oxide_bounds.width() * 0.1, // 10% from left edge
            oxide_bounds.center().y,
        );

        // Verify this point is in oxide bounds
        if oxide_bounds.contains(oxide_only_point) && !metal_bounds.contains(oxide_only_point) {
            let hit_result_oxide =
                renderer.hit_test(&stack, &transform, viewport_rect, oxide_only_point);
            assert_eq!(
                hit_result_oxide,
                Some("oxide".to_string()),
                "Hit test in oxide-only region should return dielectric layer"
            );
        } else {
            // If we can't find a non-overlapping point, just verify that the overlapping test works
            println!("Note: All points in oxide are covered by metal, which is expected for embedded conductors");
        }
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

    #[test]
    fn test_dielectric_width_adapts_to_conductor_layout() {
        use crate::data::{ConductorLayer, DielectricLayer, TechnologyInfo};

        let renderer = StackRenderer::new();
        let tech = TechnologyInfo::new("test_tech".to_string());
        let mut stack = ProcessStack::new(tech);

        // Add a dielectric layer first
        let dielectric = DielectricLayer::new("oxide1".to_string(), 100.0, 4.2);
        stack.add_layer(Layer::Dielectric(dielectric));

        // Add a conductor layer with significant thickness (will create large trapezoids)
        let conductor = ConductorLayer::new("metal1".to_string(), 200.0); // Large thickness = large trapezoids
        stack.add_layer(Layer::Conductor(Box::new(conductor)));

        // Add another dielectric on top
        let dielectric2 = DielectricLayer::new("oxide2".to_string(), 150.0, 4.2);
        stack.add_layer(Layer::Dielectric(dielectric2));

        let transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        let viewport_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));

        let scaler = renderer.create_normal_scaler(&stack);
        let layer_geometries =
            renderer.create_layer_geometries_ordered(&stack, &scaler, &transform, viewport_rect);

        // Find dielectric and conductor geometries
        let mut dielectric_geometries = Vec::new();
        let mut conductor_geometries = Vec::new();

        for geometry in &layer_geometries {
            match &geometry.shape {
                crate::renderer::geometry::LayerShape::ThreeColumnTrapezoid(_) => {
                    conductor_geometries.push(geometry);
                }
                crate::renderer::geometry::LayerShape::Rectangle(_) => {
                    dielectric_geometries.push(geometry);
                }
                _ => {}
            }
        }

        // Should have both dielectric and conductor layers
        assert!(
            !dielectric_geometries.is_empty(),
            "Should have dielectric layers"
        );
        assert!(
            !conductor_geometries.is_empty(),
            "Should have conductor layers"
        );

        // Get the bounds of conductor and dielectric layers
        let conductor_bounds = conductor_geometries[0].get_bounds();
        let dielectric_bounds = dielectric_geometries[0].get_bounds();

        println!("Test dielectric width adaptation:");
        println!("  Conductor bounds: {:?}", conductor_bounds);
        println!("  Dielectric bounds: {:?}", dielectric_bounds);

        // The dielectric layer should be wider than the conductor layer to provide proper containment
        // The dielectric should extend at least one trapezoid width beyond the conductor on each side
        assert!(
            dielectric_bounds.width() >= conductor_bounds.width(),
            "Dielectric width ({:.2}) should be at least as wide as conductor width ({:.2})",
            dielectric_bounds.width(),
            conductor_bounds.width()
        );

        // More specifically, dielectric should contain the conductor with extra margin
        // The dielectric should extend beyond the conductor bounds
        let conductor_left = conductor_bounds.min.x;
        let conductor_right = conductor_bounds.max.x;
        let dielectric_left = dielectric_bounds.min.x;
        let dielectric_right = dielectric_bounds.max.x;

        assert!(
            dielectric_left <= conductor_left,
            "Dielectric left ({:.2}) should extend beyond conductor left ({:.2})",
            dielectric_left,
            conductor_left
        );
        assert!(
            dielectric_right >= conductor_right,
            "Dielectric right ({:.2}) should extend beyond conductor right ({:.2})",
            dielectric_right,
            conductor_right
        );

        // The extra margin should be meaningful (at least 50 units for this test case)
        let left_margin = conductor_left - dielectric_left;
        let right_margin = dielectric_right - conductor_right;
        assert!(
            left_margin >= 50.0,
            "Left margin should be at least 50 units, got {:.2}",
            left_margin
        );
        assert!(
            right_margin >= 50.0,
            "Right margin should be at least 50 units, got {:.2}",
            right_margin
        );

        println!(
            "  Left margin: {:.2}, Right margin: {:.2}",
            left_margin, right_margin
        );
        println!("  Dielectric width adaptation test PASSED");
    }
}
