// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use itf_viewer::data::*;
use itf_viewer::parser::parse_itf_file;
use std::fs;

#[test]
fn test_rho_vs_si_width_thickness_parsing() {
    // Test parsing of RHO_VS_SI_WIDTH_AND_THICKNESS lookup tables
    let test_path = "tests/data/complex_test.itf";

    let content = fs::read_to_string(test_path).expect("Failed to read test ITF file");

    let result = parse_itf_file(&content);
    assert!(
        result.is_ok(),
        "Failed to parse test ITF file: {:?}",
        result.err()
    );

    let stack = result.unwrap();

    println!("Technology name: {}", stack.technology_info.name);
    println!("Total layers: {}", stack.layers.len());

    // Find a conductor layer that should have RHO_VS_SI_WIDTH_AND_THICKNESS
    let conductor_layer = stack.layers.iter().find_map(|layer| {
        if let Layer::Conductor(conductor) = layer {
            if conductor.rho_vs_si_width_thickness.is_some() {
                Some(conductor)
            } else {
                None
            }
        } else {
            None
        }
    });

    if let Some(conductor) = conductor_layer {
        println!(
            "Found conductor layer with RHO_VS_SI_WIDTH_AND_THICKNESS: {}",
            conductor.name
        );

        if let Some(rho_table) = &conductor.rho_vs_si_width_thickness {
            println!("RHO_VS_SI_WIDTH_AND_THICKNESS table found");
            println!("Widths: {} entries", rho_table.widths.len());
            println!("Thicknesses: {} entries", rho_table.spacings.len());
            println!(
                "Values: {}x{} matrix",
                rho_table.values.len(),
                rho_table.values.first().map(|v| v.len()).unwrap_or(0)
            );

            // Basic structure validation
            assert!(!rho_table.widths.is_empty(), "Should have width entries");
            assert!(
                !rho_table.spacings.is_empty(),
                "Should have thickness entries"
            );
            assert!(!rho_table.values.is_empty(), "Should have value rows");

            // Each row should have the same number of values as widths
            for (row_idx, row) in rho_table.values.iter().enumerate() {
                assert_eq!(
                    row.len(),
                    rho_table.widths.len(),
                    "Row {} should have {} values",
                    row_idx,
                    rho_table.widths.len()
                );
            }

            // Test lookup functionality
            if !rho_table.widths.is_empty() && !rho_table.spacings.is_empty() {
                let test_width = rho_table.widths[0];
                let test_thickness = rho_table.spacings[0];
                let lookup_result = rho_table.lookup(test_width, test_thickness);
                assert!(
                    lookup_result.is_some(),
                    "Lookup should work for valid coordinates"
                );
                println!(
                    "Lookup({}, {}) = {:?}",
                    test_width, test_thickness, lookup_result
                );

                // Test interpolation between valid points
                if rho_table.widths.len() > 1 && rho_table.spacings.len() > 1 {
                    let mid_width = (rho_table.widths[0] + rho_table.widths[1]) / 2.0;
                    let mid_thickness = (rho_table.spacings[0] + rho_table.spacings[1]) / 2.0;
                    let interp_result = rho_table.lookup(mid_width, mid_thickness);
                    assert!(interp_result.is_some(), "Interpolation should work");
                    println!(
                        "Interpolated lookup({}, {}) = {:?}",
                        mid_width, mid_thickness, interp_result
                    );
                }
            }

            // Test boundary extrapolation (current implementation extrapolates to boundary values)
            let oob_value = rho_table.lookup(0.001, 0.001);
            assert!(
                oob_value.is_some(),
                "Out-of-bounds lookup should extrapolate to boundary values"
            );
        }
    } else {
        println!("No conductor layer with RHO_VS_SI_WIDTH_AND_THICKNESS found in test file");
    }
}

#[test]
fn test_rho_vs_si_width_thickness_interpolation() {
    // Create a simple test case for interpolation
    let test_content = r#"
TECHNOLOGY = test_rho_interpolation
GLOBAL_TEMPERATURE = 25.0

CONDUCTOR test_metal {
    THICKNESS = 0.5
    RHO_VS_SI_WIDTH_AND_THICKNESS {
        WIDTH { 0.1 0.2 0.3 }
        THICKNESS { 0.4 0.5 0.6 }
        VALUES {
            0.010 0.015 0.020
            0.012 0.017 0.022
            0.014 0.019 0.024
        }
    }
}
"#;

    let result = parse_itf_file(test_content);
    assert!(
        result.is_ok(),
        "Failed to parse test content: {:?}",
        result.err()
    );

    let stack = result.unwrap();
    let conductor = stack
        .layers
        .iter()
        .find_map(|layer| {
            if let Layer::Conductor(c) = layer {
                Some(c)
            } else {
                None
            }
        })
        .expect("Should find conductor layer");

    let rho_table = conductor
        .rho_vs_si_width_thickness
        .as_ref()
        .expect("Should have RHO table");

    // Test exact values
    assert_eq!(rho_table.lookup(0.1, 0.4), Some(0.010));
    assert_eq!(rho_table.lookup(0.2, 0.5), Some(0.017));
    assert_eq!(rho_table.lookup(0.3, 0.6), Some(0.024));

    // Test interpolation
    let interp_result = rho_table.lookup(0.15, 0.45);
    assert!(interp_result.is_some());
    let interp_value = interp_result.unwrap();

    // Should be interpolated between surrounding values
    assert!(interp_value > 0.010 && interp_value < 0.024);
    println!("Interpolated value at (0.15, 0.45) = {}", interp_value);

    // Test boundary conditions
    assert_eq!(rho_table.lookup(0.1, 0.4), Some(0.010)); // Bottom-left corner
    assert_eq!(rho_table.lookup(0.3, 0.6), Some(0.024)); // Top-right corner

    // Test boundary extrapolation (current implementation extrapolates to boundary values)
    assert!(rho_table.lookup(0.05, 0.45).is_some()); // Below width range -> extrapolates
    assert!(rho_table.lookup(0.15, 0.35).is_some()); // Below thickness range -> extrapolates
    assert!(rho_table.lookup(0.35, 0.45).is_some()); // Above width range -> extrapolates
    assert!(rho_table.lookup(0.15, 0.65).is_some()); // Above thickness range -> extrapolates
}

#[test]
fn test_resistance_calculation_with_rho_table() {
    // Test resistance calculation using RHO_VS_SI_WIDTH_AND_THICKNESS
    let test_content = r#"
TECHNOLOGY = test_resistance
GLOBAL_TEMPERATURE = 25.0

CONDUCTOR test_metal {
    THICKNESS = 0.5
    CRT1 = 2.0e-3
    CRT2 = -4.0e-7
    RHO_VS_SI_WIDTH_AND_THICKNESS {
        WIDTH { 0.2 0.4 0.6 }
        THICKNESS { 0.4 0.5 0.6 }
        VALUES {
            0.020 0.018 0.016
            0.018 0.016 0.014
            0.016 0.014 0.012
        }
    }
}
"#;

    let result = parse_itf_file(test_content);
    assert!(
        result.is_ok(),
        "Failed to parse test content: {:?}",
        result.err()
    );

    let stack = result.unwrap();
    let conductor = stack
        .layers
        .iter()
        .find_map(|layer| {
            if let Layer::Conductor(c) = layer {
                Some(c)
            } else {
                None
            }
        })
        .expect("Should find conductor layer");

    // Test resistance calculation
    let width = 0.4;
    let length = 10.0;
    let temperature = 25.0; // Same as reference temperature
    let reference_temp = 25.0;

    let resistance = conductor.calculate_resistance(width, length, temperature, reference_temp);
    assert!(
        resistance.is_some(),
        "Resistance calculation should succeed"
    );

    let r = resistance.unwrap();
    println!("Resistance calculation test: {} ohms", r);

    // For the test values:
    // - Width = 0.4, Thickness = 0.5 -> RHO should be 0.016
    // - Volume resistance formula: R = rho * L / (W * T) = 0.016 * 10.0 / (0.4 * 0.5) = 0.016 * 10.0 / 0.2 = 0.8 ohms
    let expected_resistance = 0.016 * length / (width * conductor.thickness);
    assert!(
        (r - expected_resistance).abs() < 0.001,
        "Resistance should be approximately {} ohms, got {}",
        expected_resistance,
        r
    );

    // Test temperature dependence
    let high_temp_resistance = conductor.calculate_resistance(width, length, 85.0, reference_temp);
    assert!(
        high_temp_resistance.is_some(),
        "High temperature resistance calculation should succeed"
    );

    let r_high = high_temp_resistance.unwrap();
    println!("High temperature resistance: {} ohms", r_high);

    // At higher temperature, resistance should be higher due to positive CRT1
    assert!(r_high > r, "Resistance should increase with temperature");
}

#[test]
fn test_resistance_calculation_with_rpsq_fallback() {
    // Test that when RHO table is not available, it falls back to RPSQ
    let test_content = r#"
TECHNOLOGY = test_fallback
GLOBAL_TEMPERATURE = 25.0

CONDUCTOR test_metal_rpsq {
    THICKNESS = 0.5
    RPSQ = 0.08
    CRT1 = 2.0e-3
    CRT2 = -4.0e-7
}
"#;

    let result = parse_itf_file(test_content);
    assert!(
        result.is_ok(),
        "Failed to parse test content: {:?}",
        result.err()
    );

    let stack = result.unwrap();
    let conductor = stack
        .layers
        .iter()
        .find_map(|layer| {
            if let Layer::Conductor(c) = layer {
                Some(c)
            } else {
                None
            }
        })
        .expect("Should find conductor layer");

    // Test resistance calculation using RPSQ
    let width = 0.4;
    let length = 10.0;
    let temperature = 25.0;
    let reference_temp = 25.0;

    let resistance = conductor.calculate_resistance(width, length, temperature, reference_temp);
    assert!(
        resistance.is_some(),
        "Resistance calculation with RPSQ should succeed"
    );

    let r = resistance.unwrap();
    println!("RPSQ-based resistance: {} ohms", r);

    // Expected resistance = RPSQ * L/W = 0.08 * (10.0 / 0.4) = 0.08 * 25 = 2.0 ohms
    let expected_resistance = 0.08 * (length / width);
    assert!(
        (r - expected_resistance).abs() < 0.001,
        "Resistance should be approximately {} ohms, got {}",
        expected_resistance,
        r
    );
}

#[test]
fn test_combined_crt_and_rho_parsing() {
    // Test parsing both CRT_VS_SI_WIDTH and RHO_VS_SI_WIDTH_AND_THICKNESS in same layer
    let test_content = r#"
TECHNOLOGY = test_combined
GLOBAL_TEMPERATURE = 25.0

CONDUCTOR combined_metal {
    THICKNESS = 0.5
    RPSQ = 0.1
    CRT_VS_SI_WIDTH {
        (0.2, 2.0e-3, -3.0e-7) (0.4, 2.2e-3, -3.5e-7) (0.6, 2.4e-3, -4.0e-7)
    }
    RHO_VS_SI_WIDTH_AND_THICKNESS {
        WIDTH { 0.2 0.4 0.6 }
        THICKNESS { 0.4 0.5 0.6 }
        VALUES {
            0.020 0.018 0.016
            0.018 0.016 0.014
            0.016 0.014 0.012
        }
    }
}
"#;

    let result = parse_itf_file(test_content);
    assert!(
        result.is_ok(),
        "Failed to parse combined test content: {:?}",
        result.err()
    );

    let stack = result.unwrap();
    let conductor = stack
        .layers
        .iter()
        .find_map(|layer| {
            if let Layer::Conductor(c) = layer {
                Some(c)
            } else {
                None
            }
        })
        .expect("Should find conductor layer");

    // Verify both tables are parsed
    assert!(
        conductor.crt_vs_si_width.is_some(),
        "CRT_VS_SI_WIDTH should be parsed"
    );
    assert!(
        conductor.rho_vs_si_width_thickness.is_some(),
        "RHO_VS_SI_WIDTH_AND_THICKNESS should be parsed"
    );

    if let Some(crt_table) = &conductor.crt_vs_si_width {
        assert_eq!(
            crt_table.widths.len(),
            3,
            "CRT table should have 3 width entries"
        );

        // Test CRT lookup
        let crt_values = crt_table.lookup_crt_values(0.4);
        assert!(crt_values.is_some(), "CRT lookup should work");
        if let Some((crt1, crt2)) = crt_values {
            println!("CRT values at width 0.4: CRT1={}, CRT2={}", crt1, crt2);
            assert!(
                (crt1 - 2.2e-3).abs() < 1e-6,
                "CRT1 should be approximately 2.2e-3"
            );
            assert!(
                (crt2 - (-3.5e-7)).abs() < 1e-9,
                "CRT2 should be approximately -3.5e-7"
            );
        }
    }

    if let Some(rho_table) = &conductor.rho_vs_si_width_thickness {
        assert_eq!(
            rho_table.widths.len(),
            3,
            "RHO table should have 3 width entries"
        );
        assert_eq!(
            rho_table.spacings.len(),
            3,
            "RHO table should have 3 thickness entries"
        );

        // Test RHO lookup
        let rho_value = rho_table.lookup(0.4, 0.5);
        assert_eq!(
            rho_value,
            Some(0.016),
            "RHO lookup should return correct value"
        );
    }

    // Test resistance calculation (should use RHO table and CRT table)
    let resistance = conductor.calculate_resistance(0.4, 10.0, 25.0, 25.0);
    assert!(
        resistance.is_some(),
        "Combined resistance calculation should work"
    );

    let r = resistance.unwrap();
    println!("Combined resistance calculation: {} ohms", r);

    // Should use volume resistance formula since RHO table is available
    let expected_resistance = 0.016 * 10.0 / (0.4 * 0.5);
    assert!(
        (r - expected_resistance).abs() < 0.001,
        "Resistance should use RHO table value"
    );
}
