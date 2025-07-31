// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use itf_viewer::data::*;
use approx::assert_relative_eq;

#[test]
fn test_lookup_table_2d_functionality() {
    let table = LookupTable2D::new(
        vec![0.1, 0.2, 0.3, 0.4],
        vec![0.05, 0.1, 0.15, 0.2],
        vec![
            vec![1.0, 1.1, 1.2, 1.3],
            vec![2.0, 2.1, 2.2, 2.3],
            vec![3.0, 3.1, 3.2, 3.3],
            vec![4.0, 4.1, 4.2, 4.3],
        ],
    );
    
    // Test exact matches
    assert_eq!(table.lookup(0.1, 0.05), Some(1.0));
    assert_eq!(table.lookup(0.4, 0.2), Some(4.3));
    assert_eq!(table.lookup(0.2, 0.1), Some(2.1));
    
    // Test boundary conditions
    assert_eq!(table.lookup(0.0, 0.0), Some(1.0)); // Below minimum
    assert_eq!(table.lookup(1.0, 1.0), Some(4.3)); // Above maximum
    
    // Test interpolation behavior (should find closest match)
    let result = table.lookup(0.15, 0.075);
    assert!(result.is_some());
}

#[test]
fn test_lookup_table_1d_interpolation() {
    let table = LookupTable1D::new(
        vec![1.0, 2.0, 3.0, 4.0],
        vec![10.0, 20.0, 30.0, 40.0],
    );
    
    // Test exact matches
    assert_eq!(table.lookup(1.0), Some(10.0));
    assert_eq!(table.lookup(4.0), Some(40.0));
    
    // Test linear interpolation
    assert_relative_eq!(table.lookup(1.5).unwrap(), 15.0, epsilon = 1e-10);
    assert_relative_eq!(table.lookup(2.5).unwrap(), 25.0, epsilon = 1e-10);
    assert_relative_eq!(table.lookup(3.5).unwrap(), 35.0, epsilon = 1e-10);
    
    // Test boundary conditions
    assert_eq!(table.lookup(0.0), Some(10.0)); // Below minimum
    assert_eq!(table.lookup(5.0), Some(40.0)); // Above maximum
}

#[test]
fn test_process_variation_calculation() {
    let variation = ProcessVariation {
        density_polynomial_orders: vec![0, 1, 2],
        width_polynomial_orders: vec![0, 1],
        width_ranges: vec![1.0, 2.0],
        polynomial_coefficients: vec![
            // First width range (width <= 1.0)
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], // 6 coefficients for 3x2 polynomial
            // Second width range (1.0 < width <= 2.0)
            vec![0.5, 1.0, 1.5, 2.0, 2.5, 3.0],
        ],
    };
    
    // Test calculation for different width ranges
    let result1 = variation.calculate_thickness_variation(0.5, 0.8);
    let result2 = variation.calculate_thickness_variation(0.5, 1.5);
    
    assert_ne!(result1, result2); // Different width ranges should give different results
    
    // Test that the calculation includes all polynomial terms
    // For density=0.5, width=0.8 (first range):
    // Expected: 1.0*0.5^0*0.8^0 + 2.0*0.5^1*0.8^0 + 3.0*0.5^0*0.8^1 + 4.0*0.5^1*0.8^1 + 5.0*0.5^2*0.8^0 + 6.0*0.5^2*0.8^1
    //         = 1.0*1*1 + 2.0*0.5*1 + 3.0*1*0.8 + 4.0*0.5*0.8 + 5.0*0.25*1 + 6.0*0.25*0.8
    //         = 1.0 + 1.0 + 2.4 + 1.6 + 1.25 + 1.2 = 8.45
    assert_relative_eq!(result1, 8.45, epsilon = 1e-10);
}

#[test]
fn test_dielectric_layer_operations() {
    let layer = DielectricLayer::new("test_oxide".to_string(), 1.5, 4.2)
        .with_position(2.0)
        .with_measured_from("TOP_OF_CHIP".to_string());
    
    assert_eq!(layer.name, "test_oxide");
    assert_eq!(layer.thickness, 1.5);
    assert_eq!(layer.dielectric_constant, 4.2);
    assert_eq!(layer.z_position, 2.0);
    assert_eq!(layer.measured_from, Some("TOP_OF_CHIP".to_string()));
    
    // Test z-coordinate calculations
    assert_eq!(layer.get_bottom_z(), 2.0);
    assert_eq!(layer.get_top_z(), 3.5);
    
    // Test layer type
    assert_eq!(layer.get_layer_type(), LayerType::Dielectric);
}

#[test]
fn test_conductor_layer_operations() {
    let electrical_props = ElectricalProperties {
        crt1: Some(2.5e-3),
        crt2: Some(-4.0e-7),
        rpsq: Some(0.08),
        rpv: None,
    };
    
    let layer = ConductorLayer::new("test_metal".to_string(), 0.4)
        .with_position(1.0)
        .with_electrical_props(electrical_props)
        .with_side_tangent(0.05)
        .with_width_spacing_limits(0.1, 0.1);
    
    assert_eq!(layer.name, "test_metal");
    assert_eq!(layer.thickness, 0.4);
    assert_eq!(layer.z_position, 1.0);
    assert_eq!(layer.electrical_props.crt1, Some(2.5e-3));
    assert_eq!(layer.physical_props.side_tangent, Some(0.05));
    assert_eq!(layer.physical_props.width_min, Some(0.1));
    assert_eq!(layer.physical_props.spacing_min, Some(0.1));
    
    // Test z-coordinate calculations
    assert_eq!(layer.get_bottom_z(), 1.0);
    assert_eq!(layer.get_top_z(), 1.4);
    
    // Test trapezoid properties
    assert!(layer.is_trapezoid());
    assert_relative_eq!(layer.get_trapezoid_angle(), 0.05_f64.atan(), epsilon = 1e-10);
    
    // Test layer type
    assert_eq!(layer.get_layer_type(), LayerType::Conductor);
}

#[test]
fn test_conductor_resistance_calculation() {
    let mut layer = ConductorLayer::new("metal_test".to_string(), 0.3);
    layer.electrical_props.rpsq = Some(0.1);
    layer.electrical_props.crt1 = Some(3.0e-3);
    layer.electrical_props.crt2 = Some(-1.0e-6);
    
    // Test resistance at reference temperature
    let r_ref = layer.calculate_resistance(1.0, 10.0, 25.0, 25.0);
    assert!(r_ref.is_some());
    let r_ref = r_ref.unwrap();
    
    // Basic resistance = rpsq * length / (width * thickness) = 0.1 * 10 / (1.0 * 0.3) = 3.333...
    assert_relative_eq!(r_ref, 3.3333333333333335, epsilon = 1e-10);
    
    // Test resistance at higher temperature
    let r_hot = layer.calculate_resistance(1.0, 10.0, 125.0, 25.0);
    assert!(r_hot.is_some());
    let r_hot = r_hot.unwrap();
    
    // Should be higher due to positive temperature coefficient
    assert!(r_hot > r_ref);
    
    // Test with lookup table
    let rho_table = LookupTable2D::new(
        vec![0.5, 1.0, 2.0],
        vec![0.1, 0.2],
        vec![vec![0.12, 0.10, 0.08], vec![0.11, 0.09, 0.07]],
    );
    layer.rho_vs_width_spacing = Some(rho_table);
    
    let r_table = layer.calculate_resistance(1.0, 10.0, 25.0, 25.0);
    assert!(r_table.is_some());
    // Should use table value (0.10) instead of rpsq (0.1)
    let expected = 0.10 * 10.0 / (1.0 * 0.3);
    assert_relative_eq!(r_table.unwrap(), expected, epsilon = 1e-10);
}

#[test]
fn test_conductor_effective_width() {
    let mut layer = ConductorLayer::new("metal_test".to_string(), 0.3);
    
    // Test without etch table (should return nominal width)
    assert_eq!(layer.get_effective_width(0.2, 0.15), 0.2);
    
    // Test with etch table
    let etch_table = LookupTable2D::new(
        vec![0.1, 0.2, 0.3],
        vec![0.1, 0.15, 0.2],
        vec![
            vec![0.01, 0.015, 0.02], // For spacing 0.1
            vec![0.008, 0.012, 0.016], // For spacing 0.15
            vec![0.005, 0.01, 0.015], // For spacing 0.2
        ],
    );
    layer.etch_vs_width_spacing = Some(etch_table);
    
    // Effective width = nominal - 2 * etch_bias
    let eff_width = layer.get_effective_width(0.2, 0.15);
    let expected = 0.2 - 2.0 * 0.012;
    assert_relative_eq!(eff_width, expected, epsilon = 1e-10);
    
    // Test boundary case (should not go negative)
    let eff_width_small = layer.get_effective_width(0.01, 0.15);
    assert!(eff_width_small >= 0.0);
}

#[test]
fn test_layer_enum_operations() {
    let dielectric = Layer::Dielectric(DielectricLayer::new("oxide".to_string(), 1.0, 4.2));
    let conductor = Layer::Conductor(Box::new(ConductorLayer::new("metal".to_string(), 0.5)));
    
    // Test type checking
    assert!(dielectric.is_dielectric());
    assert!(!dielectric.is_conductor());
    assert!(conductor.is_conductor());
    assert!(!conductor.is_dielectric());
    
    // Test common interface
    assert_eq!(dielectric.name(), "oxide");
    assert_eq!(conductor.name(), "metal");
    assert_eq!(dielectric.thickness(), 1.0);
    assert_eq!(conductor.thickness(), 0.5);
    
    // Test layer type enum
    assert_eq!(dielectric.layer_type(), LayerType::Dielectric);
    assert_eq!(conductor.layer_type(), LayerType::Conductor);
    
    // Test z-position operations
    let mut dielectric_mut = dielectric.clone();
    dielectric_mut.set_z_position(2.0);
    assert_eq!(dielectric_mut.z_position(), 2.0);
    assert_eq!(dielectric_mut.get_bottom_z(), 2.0);
    assert_eq!(dielectric_mut.get_top_z(), 3.0);
}

#[test]
fn test_via_connection_operations() {
    let via = ViaConnection::new(
        "test_via".to_string(),
        "layer1".to_string(),
        "layer2".to_string(),
        0.04,
        5.0,
    ).with_geometry(1.0, 0.8);
    
    assert_eq!(via.name, "test_via");
    assert_eq!(via.from_layer, "layer1");
    assert_eq!(via.to_layer, "layer2");
    assert_eq!(via.area, 0.04);
    assert_eq!(via.resistance_per_via, 5.0);
    assert_eq!(via.z_position, 1.0);
    assert_eq!(via.height, 0.8);
    
    // Test geometric calculations
    assert_relative_eq!(via.get_via_width(), 0.2, epsilon = 1e-10); // sqrt(0.04)
    assert_eq!(via.get_via_height(), 0.8);
    assert_eq!(via.get_bottom_z(), 1.0);
    assert_eq!(via.get_top_z(), 1.8);
    
    // Test resistance calculations
    assert_eq!(via.calculate_resistance(1), 5.0);
    assert_eq!(via.calculate_resistance(2), 2.5);
    assert_eq!(via.calculate_resistance(4), 1.25);
    assert_eq!(via.calculate_resistance(0), f64::INFINITY);
    
    // Test layer connection
    assert!(via.connects_layers("layer1", "layer2"));
    assert!(via.connects_layers("layer2", "layer1")); // Bidirectional
    assert!(!via.connects_layers("layer1", "layer3"));
}

#[test]
fn test_via_type_classification() {
    // Test contact via
    let contact_via = ViaConnection::new(
        "pdiff_contact".to_string(),
        "pdiff".to_string(),
        "metal1".to_string(),
        0.01,
        50.0,
    );
    assert!(contact_via.is_contact_via());
    assert!(!contact_via.is_metal_via());
    assert_eq!(contact_via.get_via_type(), ViaType::Contact);
    
    // Test poly contact
    let poly_contact = ViaConnection::new(
        "poly_contact".to_string(),
        "poly_gate".to_string(),
        "metal1".to_string(),
        0.008,
        40.0,
    );
    assert!(poly_contact.is_contact_via());
    assert_eq!(poly_contact.get_via_type(), ViaType::Contact);
    
    // Test metal via
    let metal_via = ViaConnection::new(
        "via1".to_string(),
        "metal1".to_string(),
        "metal2".to_string(),
        0.04,
        8.0,
    );
    assert!(!metal_via.is_contact_via());
    assert!(metal_via.is_metal_via());
    assert_eq!(metal_via.get_via_type(), ViaType::Metal);
    
    // Test substrate connection
    let substrate_via = ViaConnection::new(
        "sub_contact".to_string(),
        "SUBSTRATE".to_string(),
        "pdiff".to_string(),
        0.01,
        100.0,
    );
    assert!(substrate_via.is_contact_via());
    assert_eq!(substrate_via.get_via_type(), ViaType::Contact);
    
    // Test other type
    let other_via = ViaConnection::new(
        "special".to_string(),
        "custom_layer".to_string(),
        "another_layer".to_string(),
        0.02,
        10.0,
    );
    assert!(!other_via.is_contact_via());
    assert!(!other_via.is_metal_via());
    assert_eq!(other_via.get_via_type(), ViaType::Other);
}

#[test]
fn test_via_stack_operations() {
    let mut stack = ViaStack::new();
    
    let via1 = ViaConnection::new("via1".to_string(), "metal1".to_string(), "metal2".to_string(), 0.04, 5.0);
    let via2 = ViaConnection::new("via2".to_string(), "metal2".to_string(), "metal3".to_string(), 0.04, 5.0);
    let contact = ViaConnection::new("contact".to_string(), "pdiff".to_string(), "metal1".to_string(), 0.01, 50.0);
    
    stack.add_via(via1);
    stack.add_via(via2);
    stack.add_via(contact);
    
    assert_eq!(stack.len(), 3);
    assert!(!stack.is_empty());
    
    // Test via lookup by layer
    let metal1_vias = stack.get_vias_for_layer("metal1");
    assert_eq!(metal1_vias.len(), 2); // via1 and contact
    
    let metal2_vias = stack.get_vias_for_layer("metal2");
    assert_eq!(metal2_vias.len(), 2); // via1 and via2
    
    // Test direct connection lookup
    let connection = stack.get_via_between_layers("metal1", "metal2");
    assert!(connection.is_some());
    assert_eq!(connection.unwrap().name, "via1");
    
    // Test path finding
    let path = stack.get_connection_path("pdiff", "metal3");
    assert!(path.is_some());
    let path = path.unwrap();
    assert_eq!(path.len(), 3); // contact -> via1 -> via2
    
    // Test no path case
    let no_path = stack.get_connection_path("pdiff", "unknown_layer");
    assert!(no_path.is_none());
    
    // Test same layer path
    let same_path = stack.get_connection_path("metal1", "metal1");
    assert!(same_path.is_some());
    assert!(same_path.unwrap().is_empty());
}