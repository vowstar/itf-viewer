// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use egui::{Pos2, Rect, Vec2, Shape, Stroke, Color32};
use crate::data::ConductorLayer;

#[derive(Debug, Clone)]
pub struct TrapezoidShape {
    pub bottom_left: Pos2,
    pub bottom_right: Pos2,
    pub top_left: Pos2,
    pub top_right: Pos2,
    pub fill_color: Color32,
    pub stroke: Stroke,
}

impl TrapezoidShape {
    pub fn new(
        bottom_center: Pos2,
        width: f32,
        height: f32,
        side_tangent: f32,
        fill_color: Color32,
        stroke: Stroke,
    ) -> Self {
        // Calculate trapezoid vertices
        // Positive side_tangent means top is wider (negative trapezoid)
        // Negative side_tangent means top is narrower (positive trapezoid)
        let half_width = width * 0.5;
        let width_change = height * side_tangent.abs();
        
        let (top_half_width, bottom_half_width) = if side_tangent >= 0.0 {
            // Top wider than bottom (negative trapezoid - like etched metal)
            (half_width + width_change, half_width)
        } else {
            // Top narrower than bottom (positive trapezoid - like deposited metal)
            (half_width - width_change, half_width)
        };
        
        let bottom_left = Pos2::new(bottom_center.x - bottom_half_width, bottom_center.y);
        let bottom_right = Pos2::new(bottom_center.x + bottom_half_width, bottom_center.y);
        let top_left = Pos2::new(bottom_center.x - top_half_width, bottom_center.y - height);
        let top_right = Pos2::new(bottom_center.x + top_half_width, bottom_center.y - height);
        
        Self {
            bottom_left,
            bottom_right,
            top_left,
            top_right,
            fill_color,
            stroke,
        }
    }

    pub fn from_conductor_layer(
        layer: &ConductorLayer,
        bottom_center: Pos2,
        width: f32,
        height: f32,
        fill_color: Color32,
        stroke: Stroke,
    ) -> Self {
        let side_tangent = layer.physical_props.side_tangent.unwrap_or(0.0) as f32;
        
        // Exaggerate the angle for better visualization (multiply by factor)
        let exaggerated_tangent = side_tangent * 3.0; // Make 60 degree trapezoids more visible
        
        Self::new(bottom_center, width, height, exaggerated_tangent, fill_color, stroke)
    }

    pub fn to_egui_shape(&self) -> Shape {
        let points = vec![
            self.bottom_left,
            self.bottom_right,
            self.top_right,
            self.top_left,
        ];
        
        Shape::convex_polygon(points, self.fill_color, self.stroke)
    }

    pub fn contains_point(&self, point: Pos2) -> bool {
        // Use cross product to determine if point is inside trapezoid
        self.point_in_polygon(point, &[
            self.bottom_left,
            self.bottom_right,
            self.top_right,
            self.top_left,
        ])
    }

    fn point_in_polygon(&self, point: Pos2, polygon: &[Pos2]) -> bool {
        let mut inside = false;
        let mut j = polygon.len() - 1;
        
        for i in 0..polygon.len() {
            let pi = polygon[i];
            let pj = polygon[j];
            
            if ((pi.y > point.y) != (pj.y > point.y)) &&
               (point.x < (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y) + pi.x) {
                inside = !inside;
            }
            j = i;
        }
        
        inside
    }

    pub fn get_bounds(&self) -> Rect {
        let min_x = self.bottom_left.x.min(self.top_left.x);
        let max_x = self.bottom_right.x.max(self.top_right.x);
        let min_y = self.top_left.y;
        let max_y = self.bottom_left.y;
        
        Rect::from_min_max(Pos2::new(min_x, min_y), Pos2::new(max_x, max_y))
    }
}

#[derive(Debug, Clone)]
pub struct RectangleShape {
    pub rect: Rect,
    pub fill_color: Color32,
    pub stroke: Stroke,
}

impl RectangleShape {
    pub fn new(center: Pos2, width: f32, height: f32, fill_color: Color32, stroke: Stroke) -> Self {
        let _half_width = width * 0.5;
        let _half_height = height * 0.5;
        
        let rect = Rect::from_center_size(center, Vec2::new(width, height));
        
        Self {
            rect,
            fill_color,
            stroke,
        }
    }

    pub fn from_via_dimensions(
        center: Pos2,
        via_width: f32,
        via_height: f32,
        fill_color: Color32,
        stroke: Stroke,
    ) -> Self {
        Self::new(center, via_width, via_height, fill_color, stroke)
    }

    pub fn to_egui_shape(&self) -> Shape {
        Shape::rect_filled(self.rect, 0.0, self.fill_color)
    }

    pub fn to_egui_shape_with_stroke(&self) -> Shape {
        Shape::rect_stroke(self.rect, 0.0, self.stroke)
    }

    pub fn contains_point(&self, point: Pos2) -> bool {
        self.rect.contains(point)
    }

    pub fn get_bounds(&self) -> Rect {
        self.rect
    }
}

#[derive(Debug, Clone)]
pub struct LayerGeometry {
    pub layer_name: String,
    pub z_bottom: f32,
    pub z_top: f32,
    pub shape: LayerShape,
    pub is_selected: bool,
}

#[derive(Debug, Clone)]
pub struct MultiTrapezoidShape {
    pub trapezoids: Vec<TrapezoidShape>,
}

#[derive(Debug, Clone)]
pub struct ThreeColumnTrapezoidShape {
    pub left_trapezoid: TrapezoidShape,
    pub center_trapezoid: TrapezoidShape,
    pub right_trapezoid: TrapezoidShape,
}

impl MultiTrapezoidShape {
    pub fn from_conductor_layer(
        layer: &ConductorLayer,
        bottom_center: Pos2,
        width: f32,
        height: f32,
        fill_color: Color32,
        stroke: Stroke,
        num_trapezoids: usize,
    ) -> Self {
        let num_trapezoids = num_trapezoids.max(3); // Minimum 3 trapezoids
        let mut trapezoids = Vec::new();
        
        let segment_height = height / num_trapezoids as f32;
        let side_tangent = layer.physical_props.side_tangent.unwrap_or(0.0) as f32;
        
        // Create trapezoids from bottom to top with gradual width changes
        for i in 0..num_trapezoids {
            let segment_bottom_y = bottom_center.y - (i as f32 * segment_height);
            let segment_center = Pos2::new(bottom_center.x, segment_bottom_y);
            
            // Calculate width at this height level
            let height_ratio = (i as f32) / (num_trapezoids as f32 - 1.0);
            let width_change = height * side_tangent.abs() * height_ratio;
            let segment_width = if side_tangent >= 0.0 {
                // Top wider than bottom (negative trapezoid)
                width + width_change * 2.0 // Multiply by 2 because width_change is half-width change
            } else {
                // Top narrower than bottom (positive trapezoid)
                width - width_change * 2.0
            };
            
            // Create segment-specific tangent for smooth transition
            let segment_tangent = side_tangent * 3.0; // Exaggerated for visibility
            
            let trapezoid = TrapezoidShape::new(
                segment_center,
                segment_width.max(width * 0.1), // Minimum width of 10% of original
                segment_height,
                segment_tangent,
                fill_color,
                stroke,
            );
            
            trapezoids.push(trapezoid);
        }
        
        Self { trapezoids }
    }
    
    pub fn to_egui_shapes(&self) -> Vec<Shape> {
        self.trapezoids.iter().map(|t| t.to_egui_shape()).collect()
    }
    
    pub fn contains_point(&self, point: Pos2) -> bool {
        self.trapezoids.iter().any(|t| t.contains_point(point))
    }
    
    pub fn get_bounds(&self) -> Rect {
        if self.trapezoids.is_empty() {
            return Rect::NOTHING;
        }
        
        let mut bounds = self.trapezoids[0].get_bounds();
        for trapezoid in &self.trapezoids[1..] {
            bounds = bounds.union(trapezoid.get_bounds());
        }
        bounds
    }
}

impl ThreeColumnTrapezoidShape {
    pub fn from_conductor_layer(
        layer: &ConductorLayer,
        bottom_center: Pos2,
        layer_width: f32,
        height: f32,
        fill_color: Color32,
        stroke: Stroke,
    ) -> Self {
        let side_tangent = layer.physical_props.side_tangent.unwrap_or(0.0) as f32;
        
        // 计算梯形尺寸: 长边宽度 = 高度 × 2, 短边宽度 = 高度 × 1
        let long_edge_width = height * 2.0;
        let short_edge_width = height * 1.0;
        
        // 图形被等分为4份，梯形占据3列
        let column_width = layer_width / 4.0;
        let spacing = column_width; // 间距是1/4宽度
        
        // 计算3个梯形的中心位置
        let left_center = Pos2::new(bottom_center.x - spacing, bottom_center.y);
        let center_center = bottom_center;  // 中间梯形在中心
        let right_center = Pos2::new(bottom_center.x + spacing, bottom_center.y);
        
        // 根据side_tangent确定顶部和底部宽度
        let (top_width, bottom_width) = if side_tangent >= 0.0 {
            // 顶部更宽（负梯形 - 像蚀刻金属）
            (long_edge_width, short_edge_width)
        } else {
            // 顶部更窄（正梯形 - 像沉积金属）  
            (short_edge_width, long_edge_width)
        };
        
        // 创建3个梯形，使用自定义宽度而不是基于layer_width
        let left_trapezoid = Self::create_custom_trapezoid(
            left_center, top_width, bottom_width, height, side_tangent, fill_color, stroke
        );
        
        let center_trapezoid = Self::create_custom_trapezoid(
            center_center, top_width, bottom_width, height, side_tangent, fill_color, stroke
        );
        
        let right_trapezoid = Self::create_custom_trapezoid(
            right_center, top_width, bottom_width, height, side_tangent, fill_color, stroke
        );
        
        Self {
            left_trapezoid,
            center_trapezoid,
            right_trapezoid,
        }
    }
    
    fn create_custom_trapezoid(
        bottom_center: Pos2,
        top_width: f32,
        bottom_width: f32,
        height: f32,
        _side_tangent: f32,
        fill_color: Color32,
        stroke: Stroke,
    ) -> TrapezoidShape {
        let half_top_width = top_width * 0.5;
        let half_bottom_width = bottom_width * 0.5;
        
        let bottom_left = Pos2::new(bottom_center.x - half_bottom_width, bottom_center.y);
        let bottom_right = Pos2::new(bottom_center.x + half_bottom_width, bottom_center.y);
        let top_left = Pos2::new(bottom_center.x - half_top_width, bottom_center.y - height);
        let top_right = Pos2::new(bottom_center.x + half_top_width, bottom_center.y - height);
        
        TrapezoidShape {
            bottom_left,
            bottom_right,
            top_left,
            top_right,
            fill_color,
            stroke,
        }
    }
    
    pub fn to_egui_shapes(&self) -> Vec<Shape> {
        vec![
            self.left_trapezoid.to_egui_shape(),
            self.center_trapezoid.to_egui_shape(),
            self.right_trapezoid.to_egui_shape(),
        ]
    }
    
    pub fn contains_point(&self, point: Pos2) -> bool {
        self.left_trapezoid.contains_point(point) ||
        self.center_trapezoid.contains_point(point) ||
        self.right_trapezoid.contains_point(point)
    }
    
    pub fn get_bounds(&self) -> Rect {
        let left_bounds = self.left_trapezoid.get_bounds();
        let center_bounds = self.center_trapezoid.get_bounds();
        let right_bounds = self.right_trapezoid.get_bounds();
        
        left_bounds.union(center_bounds).union(right_bounds)
    }
}

#[derive(Debug, Clone)]
pub enum LayerShape {
    Trapezoid(TrapezoidShape),
    MultiTrapezoid(MultiTrapezoidShape),
    ThreeColumnTrapezoid(ThreeColumnTrapezoidShape),
    Rectangle(RectangleShape),
}

impl LayerGeometry {
    pub fn new_trapezoid(
        layer_name: String,
        z_bottom: f32,
        z_top: f32,
        trapezoid: TrapezoidShape,
    ) -> Self {
        Self {
            layer_name,
            z_bottom,
            z_top,
            shape: LayerShape::Trapezoid(trapezoid),
            is_selected: false,
        }
    }

    pub fn new_multi_trapezoid(
        layer_name: String,
        z_bottom: f32,
        z_top: f32,
        multi_trapezoids: MultiTrapezoidShape,
    ) -> Self {
        Self {
            layer_name,
            z_bottom,
            z_top,
            shape: LayerShape::MultiTrapezoid(multi_trapezoids),
            is_selected: false,
        }
    }

    pub fn new_three_column_trapezoid(
        layer_name: String,
        z_bottom: f32,
        z_top: f32,
        three_column_trapezoids: ThreeColumnTrapezoidShape,
    ) -> Self {
        Self {
            layer_name,
            z_bottom,
            z_top,
            shape: LayerShape::ThreeColumnTrapezoid(three_column_trapezoids),
            is_selected: false,
        }
    }

    pub fn new_rectangle(
        layer_name: String,
        z_bottom: f32,
        z_top: f32,
        rectangle: RectangleShape,
    ) -> Self {
        Self {
            layer_name,
            z_bottom,
            z_top,
            shape: LayerShape::Rectangle(rectangle),
            is_selected: false,
        }
    }

    pub fn to_egui_shapes(&self) -> Vec<Shape> {
        let mut shapes = Vec::new();
        
        match &self.shape {
            LayerShape::Trapezoid(trap) => {
                shapes.push(trap.to_egui_shape());
            }
            LayerShape::MultiTrapezoid(multi_trap) => {
                shapes.extend(multi_trap.to_egui_shapes());
            }
            LayerShape::ThreeColumnTrapezoid(three_trap) => {
                shapes.extend(three_trap.to_egui_shapes());
            }
            LayerShape::Rectangle(rect) => {
                shapes.push(rect.to_egui_shape());
                if self.is_selected {
                    shapes.push(rect.to_egui_shape_with_stroke());
                }
            }
        }
        
        shapes
    }

    pub fn contains_point(&self, point: Pos2) -> bool {
        match &self.shape {
            LayerShape::Trapezoid(trap) => trap.contains_point(point),
            LayerShape::MultiTrapezoid(multi_trap) => multi_trap.contains_point(point),
            LayerShape::ThreeColumnTrapezoid(three_trap) => three_trap.contains_point(point),
            LayerShape::Rectangle(rect) => rect.contains_point(point),
        }
    }

    pub fn get_bounds(&self) -> Rect {
        match &self.shape {
            LayerShape::Trapezoid(trap) => trap.get_bounds(),
            LayerShape::MultiTrapezoid(multi_trap) => multi_trap.get_bounds(),
            LayerShape::ThreeColumnTrapezoid(three_trap) => three_trap.get_bounds(),
            LayerShape::Rectangle(rect) => rect.get_bounds(),
        }
    }

    pub fn set_selected(&mut self, selected: bool) {
        self.is_selected = selected;
    }

    pub fn get_thickness(&self) -> f32 {
        self.z_top - self.z_bottom
    }
}

#[derive(Debug, Clone)]
pub struct ViewTransform {
    pub scale: f32,
    pub offset: Vec2,
    pub viewport_size: Vec2,
}

impl ViewTransform {
    pub fn new(viewport_size: Vec2) -> Self {
        Self {
            scale: 1.0,
            offset: Vec2::ZERO,
            viewport_size,
        }
    }

    pub fn world_to_screen(&self, world_pos: Pos2) -> Pos2 {
        let scaled = Pos2::new(
            world_pos.x * self.scale,
            world_pos.y * self.scale,
        );
        
        Pos2::new(
            scaled.x + self.offset.x + self.viewport_size.x * 0.5,
            scaled.y + self.offset.y + self.viewport_size.y * 0.5,
        )
    }

    pub fn screen_to_world(&self, screen_pos: Pos2) -> Pos2 {
        let centered = Pos2::new(
            screen_pos.x - self.offset.x - self.viewport_size.x * 0.5,
            screen_pos.y - self.offset.y - self.viewport_size.y * 0.5,
        );
        
        Pos2::new(
            centered.x / self.scale,
            centered.y / self.scale,
        )
    }

    pub fn zoom(&mut self, zoom_factor: f32, zoom_center: Pos2) {
        let old_scale = self.scale;
        // Remove upper limit for zoom, only keep minimum scale
        self.scale = (self.scale * zoom_factor).max(0.01);
        
        let scale_ratio = self.scale / old_scale;
        let _world_center = self.screen_to_world(zoom_center);
        
        // Adjust offset to keep zoom center fixed
        self.offset = self.offset * scale_ratio + 
                     Vec2::new(zoom_center.x, zoom_center.y) * (1.0 - scale_ratio);
    }

    pub fn pan(&mut self, delta: Vec2) {
        self.offset += delta;
    }

    pub fn fit_bounds(&mut self, bounds: Rect, margin: f32) {
        let bounds_size = bounds.size();
        let available_size = self.viewport_size - Vec2::splat(margin * 2.0);
        
        let scale_x = available_size.x / bounds_size.x;
        let scale_y = available_size.y / bounds_size.y;
        // Remove upper limit for fit_bounds as well
        self.scale = scale_x.min(scale_y).max(0.01);
        
        let bounds_center = bounds.center();
        self.offset = Vec2::new(
            -bounds_center.x * self.scale,
            -bounds_center.y * self.scale,
        );
    }

    pub fn get_visible_world_bounds(&self) -> Rect {
        let top_left = self.screen_to_world(Pos2::ZERO);
        let bottom_right = self.screen_to_world(Pos2::new(
            self.viewport_size.x,
            self.viewport_size.y,
        ));
        
        Rect::from_two_pos(top_left, bottom_right)
    }
}

pub fn calculate_optimal_layer_width(
    stack_height: f32,
    viewport_width: f32,
    margin: f32,
) -> f32 {
    // Calculate width based on aspect ratio for good visualization
    let aspect_ratio = 2.0; // Width:Height ratio
    let available_width = viewport_width - margin * 2.0;
    let width_from_height = stack_height * aspect_ratio;
    
    width_from_height.min(available_width * 0.8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_trapezoid_creation() {
        let trapezoid = TrapezoidShape::new(
            Pos2::new(100.0, 100.0),
            20.0,
            10.0,
            0.1, // Positive side tangent (top wider)
            Color32::RED,
            Stroke::new(1.0, Color32::BLACK),
        );
        
        // Check vertices
        assert_eq!(trapezoid.bottom_left.x, 90.0);
        assert_eq!(trapezoid.bottom_right.x, 110.0);
        assert!(trapezoid.top_left.x < trapezoid.bottom_left.x);
        assert!(trapezoid.top_right.x > trapezoid.bottom_right.x);
        
        // Check height
        assert_relative_eq!(trapezoid.top_left.y, 90.0, epsilon = 1e-5);
        assert_relative_eq!(trapezoid.bottom_left.y, 100.0, epsilon = 1e-5);
    }

    #[test]
    fn test_trapezoid_negative_tangent() {
        let trapezoid = TrapezoidShape::new(
            Pos2::new(100.0, 100.0),
            20.0,
            10.0,
            -0.1, // Negative side tangent (top narrower)
            Color32::RED,
            Stroke::new(1.0, Color32::BLACK),
        );
        
        // Top should be narrower
        assert!(trapezoid.top_left.x > trapezoid.bottom_left.x);
        assert!(trapezoid.top_right.x < trapezoid.bottom_right.x);
    }

    #[test]
    fn test_rectangle_creation() {
        let rectangle = RectangleShape::new(
            Pos2::new(100.0, 100.0),
            20.0,
            10.0,
            Color32::BLUE,
            Stroke::new(1.0, Color32::BLACK),
        );
        
        let bounds = rectangle.get_bounds();
        assert_relative_eq!(bounds.width(), 20.0, epsilon = 1e-5);
        assert_relative_eq!(bounds.height(), 10.0, epsilon = 1e-5);
        assert_relative_eq!(bounds.center().x, 100.0, epsilon = 1e-5);
        assert_relative_eq!(bounds.center().y, 100.0, epsilon = 1e-5);
    }

    #[test]
    fn test_point_containment() {
        let rectangle = RectangleShape::new(
            Pos2::new(100.0, 100.0),
            20.0,
            10.0,
            Color32::BLUE,
            Stroke::new(1.0, Color32::BLACK),
        );
        
        assert!(rectangle.contains_point(Pos2::new(100.0, 100.0))); // Center
        assert!(rectangle.contains_point(Pos2::new(95.0, 98.0))); // Inside
        assert!(!rectangle.contains_point(Pos2::new(120.0, 100.0))); // Outside
        
        let trapezoid = TrapezoidShape::new(
            Pos2::new(100.0, 100.0),
            20.0,
            10.0,
            0.0, // Rectangle-like trapezoid
            Color32::RED,
            Stroke::new(1.0, Color32::BLACK),
        );
        
        assert!(trapezoid.contains_point(Pos2::new(100.0, 95.0))); // Inside
        assert!(!trapezoid.contains_point(Pos2::new(120.0, 95.0))); // Outside
    }

    #[test]
    fn test_view_transform() {
        let mut transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        
        // Test basic world to screen conversion
        let world_point = Pos2::new(10.0, 20.0);
        let screen_point = transform.world_to_screen(world_point);
        let back_to_world = transform.screen_to_world(screen_point);
        
        assert_relative_eq!(back_to_world.x, world_point.x, epsilon = 1e-3);
        assert_relative_eq!(back_to_world.y, world_point.y, epsilon = 1e-3);
        
        // Test zoom
        let initial_scale = transform.scale;
        transform.zoom(2.0, Pos2::new(400.0, 300.0)); // Zoom at center
        assert_relative_eq!(transform.scale, initial_scale * 2.0, epsilon = 1e-5);
        
        // Test pan
        let initial_offset = transform.offset;
        transform.pan(Vec2::new(10.0, 20.0));
        assert_relative_eq!(transform.offset.x, initial_offset.x + 10.0, epsilon = 1e-5);
        assert_relative_eq!(transform.offset.y, initial_offset.y + 20.0, epsilon = 1e-5);
    }

    #[test]
    fn test_fit_bounds() {
        let mut transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        let bounds = Rect::from_min_size(Pos2::new(-50.0, -30.0), Vec2::new(100.0, 60.0));
        
        transform.fit_bounds(bounds, 50.0);
        
        // Should scale to fit the bounds with margin
        assert!(transform.scale > 0.0);
        assert!(transform.scale <= 10.0);
        
        // Offset should center the bounds
        let visible_bounds = transform.get_visible_world_bounds();
        assert!(visible_bounds.contains_rect(bounds));
    }

    #[test]
    fn test_layer_geometry() {
        let rectangle = RectangleShape::new(
            Pos2::new(100.0, 100.0),
            20.0,
            10.0,
            Color32::BLUE,
            Stroke::new(1.0, Color32::BLACK),
        );
        
        let mut geometry = LayerGeometry::new_rectangle(
            "test_layer".to_string(),
            90.0,
            110.0,
            rectangle,
        );
        
        assert_eq!(geometry.layer_name, "test_layer");
        assert_eq!(geometry.get_thickness(), 20.0);
        assert!(!geometry.is_selected);
        
        geometry.set_selected(true);
        assert!(geometry.is_selected);
        
        let shapes = geometry.to_egui_shapes();
        assert!(!shapes.is_empty());
    }

    #[test]
    fn test_optimal_layer_width() {
        let width = calculate_optimal_layer_width(100.0, 800.0, 50.0);
        assert!(width > 0.0);
        assert!(width <= 800.0 - 100.0); // Should respect margins
        
        // Should be proportional to height
        let width2 = calculate_optimal_layer_width(200.0, 800.0, 50.0);
        assert!(width2 > width);
    }

    #[test]
    fn test_multi_trapezoid_creation() {
        use crate::data::ConductorLayer;
        
        let conductor = ConductorLayer::new("test_conductor".to_string(), 1.0);
        let multi_trap = MultiTrapezoidShape::from_conductor_layer(
            &conductor,
            Pos2::new(100.0, 200.0),
            50.0,
            100.0,
            Color32::RED,
            Stroke::new(1.0, Color32::BLACK),
            5,
        );
        
        // Should create exactly 5 trapezoids
        assert_eq!(multi_trap.trapezoids.len(), 5);
        
        // All trapezoids should have the same segment height
        let expected_segment_height = 100.0 / 5.0;
        for trapezoid in &multi_trap.trapezoids {
            let trap_bounds = trapezoid.get_bounds();
            assert!((trap_bounds.height() - expected_segment_height).abs() < 1e-6);
        }
        
        // Should enforce minimum of 3 trapezoids
        let multi_trap_min = MultiTrapezoidShape::from_conductor_layer(
            &conductor,
            Pos2::new(100.0, 200.0),
            50.0,
            100.0,
            Color32::RED,
            Stroke::new(1.0, Color32::BLACK),
            1, // Request only 1, should get 3
        );
        assert_eq!(multi_trap_min.trapezoids.len(), 3);
    }

    #[test]
    fn test_multi_trapezoid_bounds() {
        use crate::data::ConductorLayer;
        
        let conductor = ConductorLayer::new("test_conductor".to_string(), 1.0);
        let multi_trap = MultiTrapezoidShape::from_conductor_layer(
            &conductor,
            Pos2::new(100.0, 200.0),
            50.0,
            100.0,
            Color32::RED,
            Stroke::new(1.0, Color32::BLACK),
            3,
        );
        
        let bounds = multi_trap.get_bounds();
        
        // Bounds should encompass all trapezoids
        assert!(bounds.width() > 0.0);
        assert!(bounds.height() > 0.0);
        
        // Each individual trapezoid should be within the overall bounds
        for trapezoid in &multi_trap.trapezoids {
            let trap_bounds = trapezoid.get_bounds();
            assert!(bounds.contains_rect(trap_bounds));
        }
    }

    #[test]
    fn test_multi_trapezoid_point_containment() {
        use crate::data::ConductorLayer;
        
        let conductor = ConductorLayer::new("test_conductor".to_string(), 1.0);
        let multi_trap = MultiTrapezoidShape::from_conductor_layer(
            &conductor,
            Pos2::new(100.0, 200.0),
            50.0,
            100.0,
            Color32::RED,
            Stroke::new(1.0, Color32::BLACK),
            3,
        );
        
        let bounds = multi_trap.get_bounds();
        let center_point = bounds.center();
        
        // Point at center should be contained
        assert!(multi_trap.contains_point(center_point));
        
        // Point far outside should not be contained
        let outside_point = Pos2::new(bounds.max.x + 100.0, bounds.max.y + 100.0);
        assert!(!multi_trap.contains_point(outside_point));
    }

    #[test]
    fn test_multi_trapezoid_shapes_generation() {
        use crate::data::ConductorLayer;
        
        let conductor = ConductorLayer::new("test_conductor".to_string(), 1.0);
        let multi_trap = MultiTrapezoidShape::from_conductor_layer(
            &conductor,
            Pos2::new(100.0, 200.0),
            50.0,
            100.0,
            Color32::RED,
            Stroke::new(1.0, Color32::BLACK),
            4,
        );
        
        let shapes = multi_trap.to_egui_shapes();
        
        // Should generate one shape per trapezoid
        assert_eq!(shapes.len(), 4);
        
        // All shapes should be valid (not empty)
        for shape in shapes {
            match shape {
                Shape::Mesh(_) => {}, // Valid shape type for convex polygons
                Shape::Path(_) => {}, // Also valid
                _ => panic!("Unexpected shape type for trapezoid"),
            }
        }
    }

    #[test]
    fn test_layer_geometry_multi_trapezoid() {
        use crate::data::ConductorLayer;
        
        let conductor = ConductorLayer::new("test_conductor".to_string(), 1.0);
        let multi_trap = MultiTrapezoidShape::from_conductor_layer(
            &conductor,
            Pos2::new(100.0, 200.0),
            50.0,
            100.0,
            Color32::RED,
            Stroke::new(1.0, Color32::BLACK),
            3,
        );
        
        let layer_geometry = LayerGeometry::new_multi_trapezoid(
            "test_layer".to_string(),
            0.0,
            1.0,
            multi_trap,
        );
        
        // Test basic properties
        assert_eq!(layer_geometry.layer_name, "test_layer");
        assert_eq!(layer_geometry.z_bottom, 0.0);
        assert_eq!(layer_geometry.z_top, 1.0);
        assert!(!layer_geometry.is_selected);
        
        // Test shape generation
        let shapes = layer_geometry.to_egui_shapes();
        assert_eq!(shapes.len(), 3); // Should generate 3 shapes for 3 trapezoids
        
        // Test bounds
        let bounds = layer_geometry.get_bounds();
        assert!(bounds.width() > 0.0);
        assert!(bounds.height() > 0.0);
        
        // Test selection
        let mut layer_geometry_mut = layer_geometry.clone();
        layer_geometry_mut.set_selected(true);
        assert!(layer_geometry_mut.is_selected);
    }
}