// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use itf_viewer::*;
use std::fs;

#[test]
fn test_full_parsing_workflow() {
    let content = fs::read_to_string("tests/data/simple_1p3m.itf")
        .expect("Failed to read test file");
    
    // Test the full parsing workflow
    let stack = parse_itf_file(&content).expect("Failed to parse ITF file");
    
    // Validate the parsed stack
    assert!(stack.validate_stack().is_ok());
    
    // Test process summary
    let summary = stack.get_process_summary();
    assert_eq!(summary.technology_name, "test_1p3m_generic");
    assert_eq!(summary.conductor_layers, 4); // poly + metal1 + metal2 + metal3
    assert_eq!(summary.dielectric_layers, 5); // substrate_oxide + ild1 + ild2 + ild3 + passivation
    assert_eq!(summary.via_connections, 3);
    
    // Test layer access
    assert!(stack.get_layer("poly").is_some());
    assert!(stack.get_layer("metal1").is_some());
    assert!(stack.get_layer("metal2").is_some());
    assert!(stack.get_layer("metal3").is_some());
    
    // Test via connections
    assert!(stack.via_stack.get_via_between_layers("poly", "metal1").is_some());
    assert!(stack.via_stack.get_via_between_layers("metal1", "metal2").is_some());
    assert!(stack.via_stack.get_via_between_layers("metal2", "metal3").is_some());
}

#[test]
fn test_complex_stack_parsing() {
    let content = fs::read_to_string("tests/data/via_connections.itf")
        .expect("Failed to read test file");
    
    let stack = parse_itf_file(&content).expect("Failed to parse complex ITF file");
    
    // Validate complex stack
    assert!(stack.validate_stack().is_ok());
    
    let summary = stack.get_process_summary();
    assert_eq!(summary.technology_name, "via_test_generic");
    assert!(summary.total_layers > 5);
    assert!(summary.via_connections > 3);
    
    // Test substrate and diffusion layers
    assert!(stack.get_layer("substrate").is_some());
    assert!(stack.get_layer("pdiff").is_some());
    assert!(stack.get_layer("ndiff").is_some());
    
    // Test via path finding
    let path = stack.via_stack.get_connection_path("pdiff", "top_metal");
    assert!(path.is_some());
    assert!(!path.unwrap().is_empty());
}

#[test]
fn test_dielectric_only_parsing() {
    let content = fs::read_to_string("tests/data/basic_dielectric.itf")
        .expect("Failed to read test file");
    
    let stack = parse_itf_file(&content).expect("Failed to parse dielectric-only ITF");
    
    assert!(stack.validate_stack().is_ok());
    
    let summary = stack.get_process_summary();
    assert_eq!(summary.technology_name, "dielectric_test_generic");
    assert_eq!(summary.conductor_layers, 0);
    assert_eq!(summary.dielectric_layers, 3);
    assert_eq!(summary.via_connections, 0);
    
    // All layers should be dielectric
    for layer in &stack.layers {
        assert!(layer.is_dielectric());
    }
}

#[test]
fn test_layer_positioning() {
    let content = fs::read_to_string("tests/data/simple_1p3m.itf")
        .expect("Failed to read test file");
    
    let stack = parse_itf_file(&content).expect("Failed to parse ITF file");
    
    // Test that layers are properly positioned
    let mut previous_top = 0.0;
    for layer in &stack.layers {
        let bottom = layer.get_bottom_z();
        let top = layer.get_top_z();
        
        // Each layer should start where the previous one ended
        assert!((bottom - previous_top).abs() < 1e-10, 
               "Layer {} positioning error: bottom={}, expected={}", 
               layer.name(), bottom, previous_top);
        
        // Layer should have positive thickness
        assert!(top > bottom, "Layer {} has invalid thickness", layer.name());
        
        previous_top = top;
    }
    
    // Total height should match sum of layer thicknesses
    let total_thickness: f64 = stack.layers.iter().map(|l| l.thickness()).sum();
    assert!((stack.get_total_height() - total_thickness).abs() < 1e-10);
}

#[test]
fn test_via_positioning() {
    let content = fs::read_to_string("tests/data/simple_1p3m.itf")
        .expect("Failed to read test file");
    
    let stack = parse_itf_file(&content).expect("Failed to parse ITF file");
    
    // Test via positioning
    for via in stack.via_stack.iter() {
        let from_layer = stack.get_layer(&via.from_layer);
        let to_layer = stack.get_layer(&via.to_layer);
        
        assert!(from_layer.is_some(), "Via {} references unknown from layer {}", 
               via.name, via.from_layer);
        assert!(to_layer.is_some(), "Via {} references unknown to layer {}", 
               via.name, via.to_layer);
        
        let from_layer = from_layer.unwrap();
        let to_layer = to_layer.unwrap();
        
        // Via should span between the layers
        let expected_bottom = from_layer.get_top_z().min(to_layer.get_bottom_z());
        let expected_top = from_layer.get_top_z().max(to_layer.get_bottom_z());
        
        assert!((via.get_bottom_z() - expected_bottom).abs() < 1e-6,
               "Via {} bottom positioning error", via.name);
        assert!((via.get_top_z() - expected_top).abs() < 1e-6,
               "Via {} top positioning error", via.name);
    }
}

#[test]
fn test_electrical_property_extraction() {
    let content = fs::read_to_string("tests/data/simple_1p3m.itf")
        .expect("Failed to read test file");
    
    let stack = parse_itf_file(&content).expect("Failed to parse ITF file");
    
    // Test that electrical properties are properly extracted
    if let Some(Layer::Conductor(metal1)) = stack.get_layer("metal1") {
        assert!(metal1.electrical_props.crt1.is_some());
        assert!(metal1.electrical_props.crt2.is_some());
        assert!(metal1.electrical_props.rpsq.is_some());
        
        // Test resistance calculation
        let resistance = metal1.calculate_resistance(0.2, 10.0, 85.0, 25.0);
        assert!(resistance.is_some());
        assert!(resistance.unwrap() > 0.0);
        
        // Test lookup tables if present
        if let Some(ref rho_table) = metal1.rho_vs_width_spacing {
            assert!(!rho_table.widths.is_empty());
            assert!(!rho_table.spacings.is_empty());
            assert!(!rho_table.values.is_empty());
            
            let rho_value = rho_table.lookup(0.2, 0.2);
            assert!(rho_value.is_some());
        }
    }
}

#[test]
fn test_physical_property_extraction() {
    let content = fs::read_to_string("tests/data/simple_1p3m.itf")
        .expect("Failed to read test file");
    
    let stack = parse_itf_file(&content).expect("Failed to parse ITF file");
    
    // Test physical properties
    if let Some(Layer::Conductor(poly)) = stack.get_layer("poly") {
        assert!(poly.physical_props.width_min.is_some());
        assert!(poly.physical_props.spacing_min.is_some());
        assert!(poly.physical_props.side_tangent.is_some());
        
        // Test trapezoid detection
        assert!(poly.is_trapezoid());
        
        let angle = poly.get_trapezoid_angle();
        assert!(angle != 0.0);
    }
    
    // Test dielectric properties
    if let Some(Layer::Dielectric(oxide)) = stack.get_layer("substrate_oxide") {
        assert!(oxide.dielectric_constant > 1.0);
        assert!(oxide.thickness > 0.0);
    }
}

#[test]
fn test_layer_filtering() {
    let content = fs::read_to_string("tests/data/via_connections.itf")
        .expect("Failed to read test file");
    
    let stack = parse_itf_file(&content).expect("Failed to parse ITF file");
    
    // Test layer filtering methods
    let conductors = stack.get_conductor_layers();
    let dielectrics = stack.get_dielectric_layers();
    let metals = stack.get_metal_layers();
    
    assert!(!conductors.is_empty());
    assert!(!dielectrics.is_empty());
    assert!(!metals.is_empty());
    
    // Total should match
    assert_eq!(conductors.len() + dielectrics.len(), stack.get_layer_count());
    
    // Metals should be subset of conductors
    assert!(metals.len() <= conductors.len());
    
    // Test z-range filtering
    let mid_layers = stack.get_layers_in_z_range(1.0, 5.0);
    for layer in mid_layers {
        let bottom = layer.get_bottom_z();
        let top = layer.get_top_z();
        assert!(bottom < 5.0 && top > 1.0, 
               "Layer {} not in z-range: {}-{}", layer.name(), bottom, top);
    }
}

#[test]
fn test_error_handling() {
    // Test empty content
    let result = parse_itf_file("");
    assert!(result.is_err());
    
    // Test invalid syntax
    let result = parse_itf_file("invalid content { } }");
    assert!(result.is_err());
    
    // Test missing required fields
    let result = parse_itf_file("TECHNOLOGY = test");
    assert!(result.is_err());
    
    // Test malformed layer definition
    let result = parse_itf_file("TECHNOLOGY = test\nDIELECTRIC layer");
    assert!(result.is_err());
}

#[test]
fn test_library_functions() {
    // Test library info functions
    let info = get_library_info();
    assert!(info.contains("itf-viewer"));
    assert!(info.contains("0.1.0"));
    
    // Test validation function
    let valid_content = "TECHNOLOGY = test\nDIELECTRIC layer {THICKNESS=1.0 ER=4.2}";
    assert!(validate_itf_content(valid_content));
    
    let invalid_content = "not itf content";
    assert!(!validate_itf_content(invalid_content));
    
    // Test configuration
    let config = get_default_config();
    assert!(config.window_width > 0.0);
    assert!(config.window_height > 0.0);
    assert!(config.show_dimensions);
}

#[test]
fn test_file_parsing_from_path() {
    // Test parsing from file path
    let stack = parse_itf_from_file("tests/data/simple_1p3m.itf")
        .expect("Failed to parse from file path");
    
    assert_eq!(stack.technology_info.name, "test_1p3m_generic");
    assert!(stack.get_layer_count() > 0);
    
    // Test non-existent file
    let result = parse_itf_from_file("nonexistent.itf");
    assert!(result.is_err());
}

#[test]
fn test_process_summary_consistency() {
    let content = fs::read_to_string("tests/data/simple_1p3m.itf")
        .expect("Failed to read test file");
    
    let stack = parse_itf_file(&content).expect("Failed to parse ITF file");
    let summary = stack.get_process_summary();
    
    // Verify summary consistency with actual data
    assert_eq!(summary.total_layers, stack.get_layer_count());
    assert_eq!(summary.conductor_layers, stack.get_conductor_count());
    assert_eq!(summary.dielectric_layers, stack.get_dielectric_count());
    assert_eq!(summary.via_connections, stack.get_via_count());
    assert_eq!(summary.total_height, stack.get_total_height());
    assert_eq!(summary.global_temperature, stack.technology_info.global_temperature);
}

#[cfg(test)]
mod renderer_tests {
    use super::*;
    use itf_viewer::renderer::{ColorScheme, ViewTransform, StackRenderer, TrapezoidShape, RectangleShape};
    use egui::Vec2;

    #[test]
    fn test_color_scheme() {
        let scheme = ColorScheme::new();
        
        // Test dielectric colors
        let content = std::fs::read_to_string("tests/data/basic_dielectric.itf")
            .expect("Failed to read test file");
        let stack = parse_itf_file(&content).expect("Failed to parse ITF file");
        
        for (i, layer) in stack.layers.iter().enumerate() {
            let color = scheme.get_layer_color(layer, i);
            assert_ne!(color, egui::Color32::TRANSPARENT);
            
            let alpha = scheme.get_layer_alpha(layer, false);
            assert!(alpha > 0);
            
            let alpha_selected = scheme.get_layer_alpha(layer, true);
            assert_eq!(alpha_selected, 255);
        }
    }

    #[test]
    fn test_view_transform() {
        let mut transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        
        // Test zoom
        let initial_scale = transform.scale;
        transform.zoom(2.0, egui::Pos2::new(400.0, 300.0));
        assert!(transform.scale > initial_scale);
        
        // Test pan
        let initial_offset = transform.offset;
        transform.pan(Vec2::new(50.0, 25.0));
        assert_ne!(transform.offset, initial_offset);
        
        // Test coordinate conversion
        let world_point = egui::Pos2::new(10.0, 20.0);
        let screen_point = transform.world_to_screen(world_point);
        let back_to_world = transform.screen_to_world(screen_point);
        
        assert!((back_to_world.x - world_point.x).abs() < 0.1);
        assert!((back_to_world.y - world_point.y).abs() < 0.1);
    }

    #[test]
    fn test_geometry_creation() {
        use egui::{Pos2, Color32, Stroke};
        
        // Test trapezoid
        let trapezoid = TrapezoidShape::new(
            Pos2::new(100.0, 100.0),
            20.0,
            10.0,
            0.1,
            Color32::RED,
            Stroke::new(1.0, Color32::BLACK),
        );
        
        let bounds = trapezoid.get_bounds();
        assert!(bounds.width() > 0.0);
        assert!(bounds.height() > 0.0);
        
        // Test rectangle
        let rectangle = RectangleShape::new(
            Pos2::new(100.0, 100.0),
            20.0,
            10.0,
            Color32::BLUE,
            Stroke::new(1.0, Color32::BLACK),
        );
        
        let bounds = rectangle.get_bounds();
        assert!((bounds.width() - 20.0).abs() < 0.01);
        assert!((bounds.height() - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_stack_renderer() {
        let content = std::fs::read_to_string("tests/data/simple_1p3m.itf")
            .expect("Failed to read test file");
        let stack = parse_itf_file(&content).expect("Failed to parse ITF file");
        
        let renderer = StackRenderer::new();
        let transform = ViewTransform::new(Vec2::new(800.0, 600.0));
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800.0, 600.0));
        
        let shapes = renderer.render_stack(&stack, &transform, viewport);
        assert!(!shapes.is_empty());
        
        let bounds = renderer.get_stack_bounds(&stack);
        assert!(bounds.width() > 0.0 && bounds.height() > 0.0);
        assert!(bounds.height() > 0.0);
    }
}

#[test]
fn test_all_test_files_parse() {
    let test_files = [
        "tests/data/simple_1p3m.itf",
        "tests/data/basic_dielectric.itf", 
        "tests/data/via_connections.itf",
    ];
    
    for file_path in &test_files {
        let content = fs::read_to_string(file_path)
            .unwrap_or_else(|_| panic!("Failed to read {file_path}"));
        
        let stack = parse_itf_file(&content)
            .unwrap_or_else(|e| panic!("Failed to parse {file_path}: {e}"));
        
        // Basic validation
        assert!(stack.validate_stack().is_ok(), "Stack validation failed for {file_path}");
        assert!(stack.get_layer_count() > 0, "No layers found in {file_path}");
        
        let summary = stack.get_process_summary();
        assert!(!summary.technology_name.is_empty(), "Empty technology name in {file_path}");
        assert!(summary.total_height > 0.0, "Invalid total height in {file_path}");
    }
}