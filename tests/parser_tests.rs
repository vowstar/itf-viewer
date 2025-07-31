// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use itf_viewer::parser::*;
use itf_viewer::data::*;
use std::fs;

#[test]
fn test_parse_simple_1p3m() {
    let content = fs::read_to_string("tests/data/simple_1p3m.itf")
        .expect("Failed to read test file");
    
    let result = parse_itf_file(&content);
    assert!(result.is_ok(), "Failed to parse simple_1p3m.itf: {:?}", result.err());
    
    let stack = result.unwrap();
    
    // Test technology info
    assert_eq!(stack.technology_info.name, "test_1p3m_generic");
    assert_eq!(stack.technology_info.global_temperature, Some(25.0));
    assert_eq!(stack.technology_info.reference_direction, Some("VERTICAL".to_string()));
    
    // Test layer count
    assert_eq!(stack.get_layer_count(), 7); // 4 dielectrics + 3 conductors
    assert_eq!(stack.get_conductor_count(), 3);
    assert_eq!(stack.get_dielectric_count(), 4);
    
    // Test specific layers
    let poly = stack.get_layer("poly");
    assert!(poly.is_some());
    assert!(poly.unwrap().is_conductor());
    
    let metal1 = stack.get_layer("metal1");
    assert!(metal1.is_some());
    if let Layer::Conductor(m1) = metal1.unwrap() {
        assert_eq!(m1.thickness, 0.3);
        assert_eq!(m1.electrical_props.crt1, Some(2.5e-3));
        assert!(m1.rho_vs_width_spacing.is_some());
        assert!(m1.etch_vs_width_spacing.is_some());
    }
    
    // Test via connections
    assert_eq!(stack.get_via_count(), 3);
    let via1 = stack.via_stack.get_via_between_layers("metal1", "metal2");
    assert!(via1.is_some());
    assert_eq!(via1.unwrap().name, "via1");
}

#[test]
fn test_parse_basic_dielectric() {
    let content = fs::read_to_string("tests/data/basic_dielectric.itf")
        .expect("Failed to read test file");
    
    let result = parse_itf_file(&content);
    assert!(result.is_ok(), "Failed to parse basic_dielectric.itf: {:?}", result.err());
    
    let stack = result.unwrap();
    
    // Test technology info with more parameters
    assert_eq!(stack.technology_info.name, "dielectric_test_generic");
    assert_eq!(stack.technology_info.global_temperature, Some(75.0));
    assert_eq!(stack.technology_info.reference_direction, Some("HORIZONTAL".to_string()));
    assert_eq!(stack.technology_info.background_er, Some(3.8));
    assert_eq!(stack.technology_info.use_si_density, Some(true));
    
    // Test only dielectric layers
    assert_eq!(stack.get_layer_count(), 3);
    assert_eq!(stack.get_conductor_count(), 0);
    assert_eq!(stack.get_dielectric_count(), 3);
    
    // Test specific dielectric properties
    let bottom = stack.get_layer("bottom_oxide");
    assert!(bottom.is_some());
    if let Layer::Dielectric(d) = bottom.unwrap() {
        assert_eq!(d.thickness, 1.2);
        assert_eq!(d.dielectric_constant, 4.1);
        assert!(d.measured_from.is_some());
    }
    
    let middle = stack.get_layer("middle_nitride");
    assert!(middle.is_some());
    if let Layer::Dielectric(d) = middle.unwrap() {
        assert_eq!(d.thickness, 0.8);
        assert_eq!(d.dielectric_constant, 7.5);
        assert_eq!(d.sw_t, Some(0.4));
        assert_eq!(d.tw_t, Some(0.6));
    }
}

#[test]
fn test_parse_via_connections() {
    let content = fs::read_to_string("tests/data/via_connections.itf")
        .expect("Failed to read test file");
    
    let result = parse_itf_file(&content);
    assert!(result.is_ok(), "Failed to parse via_connections.itf: {:?}", result.err());
    
    let stack = result.unwrap();
    
    // Test complex stack
    assert_eq!(stack.get_layer_count(), 12); // Multiple layers
    assert_eq!(stack.get_via_count(), 6);
    
    // Test substrate contacts
    let pdiff_contact = stack.via_stack.get_via_between_layers("pdiff", "metal1");
    assert!(pdiff_contact.is_some());
    assert!(pdiff_contact.unwrap().is_contact_via());
    
    let ndiff_contact = stack.via_stack.get_via_between_layers("ndiff", "metal1");
    assert!(ndiff_contact.is_some());
    assert!(ndiff_contact.unwrap().is_contact_via());
    
    // Test metal vias
    let via1 = stack.via_stack.get_via_between_layers("metal1", "metal2");
    assert!(via1.is_some());
    assert!(via1.unwrap().is_metal_via());
    
    // Test via path finding
    let path = stack.via_stack.get_connection_path("pdiff", "top_metal");
    assert!(path.is_some());
    let path = path.unwrap();
    assert_eq!(path.len(), 4); // pdiff->metal1->metal2->metal3->top_metal
}

#[test]
fn test_stack_validation() {
    let content = fs::read_to_string("tests/data/simple_1p3m.itf")
        .expect("Failed to read test file");
    
    let result = parse_itf_file(&content);
    assert!(result.is_ok());
    
    let stack = result.unwrap();
    assert!(stack.validate_stack().is_ok());
    
    // Test layer positioning
    let layers = &stack.layers;
    for i in 1..layers.len() {
        let prev_top = layers[i-1].get_top_z();
        let curr_bottom = layers[i].get_bottom_z();
        assert!((prev_top - curr_bottom).abs() < 1e-10, 
               "Layer {} position mismatch: prev_top={}, curr_bottom={}", 
               layers[i].name(), prev_top, curr_bottom);
    }
}

#[test]
fn test_process_summary() {
    let content = fs::read_to_string("tests/data/simple_1p3m.itf")
        .expect("Failed to read test file");
    
    let stack = parse_itf_file(&content).unwrap();
    let summary = stack.get_process_summary();
    
    assert_eq!(summary.technology_name, "test_1p3m_generic");
    assert_eq!(summary.total_layers, 7);
    assert_eq!(summary.conductor_layers, 3);
    assert_eq!(summary.dielectric_layers, 4);
    assert_eq!(summary.metal_layers, 3); // poly, metal1, metal2, metal3
    assert_eq!(summary.via_connections, 3);
    assert_eq!(summary.global_temperature, Some(25.0));
    assert!(summary.total_height > 0.0);
}

#[test]
fn test_layer_filtering() {
    let content = fs::read_to_string("tests/data/via_connections.itf")
        .expect("Failed to read test file");
    
    let stack = parse_itf_file(&content).unwrap();
    
    // Test layer filtering by type
    let conductors = stack.get_conductor_layers();
    let dielectrics = stack.get_dielectric_layers();
    
    assert_eq!(conductors.len() + dielectrics.len(), stack.get_layer_count());
    
    // Test layer filtering by z-range
    let mid_layers = stack.get_layers_in_z_range(1.0, 3.0);
    assert!(!mid_layers.is_empty());
    
    for layer in mid_layers {
        let bottom_z = layer.get_bottom_z();
        let top_z = layer.get_top_z();
        assert!(bottom_z < 3.0 && top_z > 1.0, 
               "Layer {} not in range: bottom={}, top={}", 
               layer.name(), bottom_z, top_z);
    }
}

#[test]
fn test_conductor_properties() {
    let content = fs::read_to_string("tests/data/simple_1p3m.itf")
        .expect("Failed to read test file");
    
    let stack = parse_itf_file(&content).unwrap();
    
    if let Some(Layer::Conductor(metal1)) = stack.get_layer("metal1") {
        // Test trapezoid detection
        assert!(metal1.is_trapezoid());
        assert_eq!(metal1.physical_props.side_tangent, Some(0.05));
        
        // Test resistance calculation
        let resistance = metal1.calculate_resistance(0.2, 10.0, 85.0, 25.0);
        assert!(resistance.is_some());
        let r = resistance.unwrap();
        assert!(r > 0.0);
        
        // Test effective width calculation
        let eff_width = metal1.get_effective_width(0.2, 0.15);
        assert!(eff_width <= 0.2); // Should be reduced due to etch
        assert!(eff_width > 0.0);
        
        // Test lookup table functionality
        if let Some(rho_table) = &metal1.rho_vs_width_spacing {
            let rho = rho_table.lookup(0.2, 0.2);
            assert!(rho.is_some());
            assert!(rho.unwrap() > 0.0);
        }
    }
}

#[test]
fn test_via_properties() {
    let content = fs::read_to_string("tests/data/via_connections.itf")
        .expect("Failed to read test file");
    
    let stack = parse_itf_file(&content).unwrap();
    
    // Test via type detection
    for via in stack.via_stack.iter() {
        match via.get_via_type() {
            ViaType::Contact => {
                assert!(via.name.contains("contact") || 
                       via.from_layer.contains("diff") || 
                       via.from_layer.contains("poly"));
            },
            ViaType::Metal => {
                assert!(via.name.starts_with("via") || 
                       (via.from_layer.starts_with("metal") && 
                        via.to_layer.starts_with("metal")));
            },
            ViaType::Other => {},
        }
        
        // Test via geometry calculations
        assert!(via.get_via_width() > 0.0);
        assert_eq!(via.get_via_width(), via.area.sqrt());
        
        // Test resistance calculation
        assert_eq!(via.calculate_resistance(1), via.resistance_per_via);
        assert_eq!(via.calculate_resistance(2), via.resistance_per_via / 2.0);
        assert_eq!(via.calculate_resistance(0), f64::INFINITY);
    }
}

#[test]
fn test_parse_generic_simple() {
    let content = fs::read_to_string("tests/data/simple_stack.itf")
        .expect("Failed to read simple stack test file");
    
    let result = parse_itf_file(&content);
    assert!(result.is_ok(), "Failed to parse simple stack test: {:?}", result.err());
    
    let stack = result.unwrap();
    
    // Test technology info
    assert_eq!(stack.technology_info.name, "generic_1p3m");
    assert_eq!(stack.technology_info.global_temperature, Some(25.0));
    assert_eq!(stack.technology_info.reference_direction, Some("VERTICAL".to_string()));
    
    // Test layer structure
    assert!(stack.get_layer_count() >= 7);
    assert!(stack.get_conductor_count() >= 4); // poly + 3 metals
    assert!(stack.get_dielectric_count() >= 3);
    
    // Test via connections
    assert_eq!(stack.get_via_count(), 3);
    
    // Test specific layers exist
    assert!(stack.get_layer("poly").is_some());
    assert!(stack.get_layer("metal1").is_some());
    assert!(stack.get_layer("metal2").is_some());
    assert!(stack.get_layer("metal3").is_some());
}

#[test]
fn test_parse_generic_complex() {
    let content = fs::read_to_string("tests/data/complex_stack.itf")
        .expect("Failed to read complex stack test file");
    
    let result = parse_itf_file(&content);
    assert!(result.is_ok(), "Failed to parse complex stack test: {:?}", result.err());
    
    let stack = result.unwrap();
    
    // Test technology info
    assert_eq!(stack.technology_info.name, "generic_complex");
    assert_eq!(stack.technology_info.global_temperature, Some(85.0));
    assert_eq!(stack.technology_info.reference_direction, Some("VERTICAL".to_string()));
    assert_eq!(stack.technology_info.background_er, Some(1.0));
    
    // Test complex layer structure
    assert!(stack.get_layer_count() > 10);
    assert!(stack.get_conductor_count() >= 6);
    assert!(stack.get_dielectric_count() >= 8);
    
    // Test via connections
    assert_eq!(stack.get_via_count(), 7);
    
    // Test that key layers exist
    assert!(stack.get_layer("m1").is_some());
    assert!(stack.get_layer("m2").is_some());
    assert!(stack.get_layer("m3").is_some());
}

#[test]
fn test_parse_generic_minimal() {
    let content = fs::read_to_string("tests/data/minimal.itf")
        .expect("Failed to read minimal test file");
    
    let result = parse_itf_file(&content);
    assert!(result.is_ok(), "Failed to parse minimal test: {:?}", result.err());
    
    let stack = result.unwrap();
    
    // Test minimal structure
    assert_eq!(stack.technology_info.name, "test_minimal");
    assert_eq!(stack.get_layer_count(), 3);
    assert_eq!(stack.get_conductor_count(), 1);
    assert_eq!(stack.get_dielectric_count(), 2);
    assert_eq!(stack.get_via_count(), 0);
}

#[test] 
fn test_error_handling() {
    // Test empty file
    let result = parse_itf_file("");
    assert!(result.is_err());
    
    // Test malformed content
    let bad_content = "INVALID SYNTAX {{{ }}}";
    let result = parse_itf_file(bad_content);
    assert!(result.is_err());
    
    // Test missing required fields
    let incomplete_content = "TECHNOLOGY = test\nDIELECTRIC layer {}";
    let result = parse_itf_file(incomplete_content);
    assert!(result.is_err());
}