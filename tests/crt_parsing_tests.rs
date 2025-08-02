// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use approx::assert_relative_eq;
use itf_viewer::data::*;
use itf_viewer::parser::parse_itf_file;

#[test]
fn test_crt_vs_si_width_parsing() {
    let itf_content = r#"
TECHNOLOGY = test_tech
GLOBAL_TEMPERATURE = 25.0

CONDUCTOR M8 {
    THICKNESS = 0.885450
    CRT1 = 3.890e-03
    CRT2 = -1.500e-07
    WMIN = 0.36
    SMIN = 0.36
    CRT_VS_SI_WIDTH {
        (0.3900, 3.6490e-03, -8.5347e-07)
        (0.4572, 3.6834e-03, -8.5317e-07)  
        (0.5520, 3.7122e-03, -8.2474e-07)
        (0.7000, 3.7416e-03, -8.9018e-07)
        (1.0630, 3.7820e-03, -9.4955e-07)
        (1.3347, 3.7960e-03, -7.8620e-07)
        (4.4036, 3.8055e-03, -4.7080e-07)
        (6.5707, 3.8055e-03, -4.7080e-07)
    }
}
"#;

    let result = parse_itf_file(itf_content);
    assert!(result.is_ok());

    let stack = result.unwrap();
    assert_eq!(stack.layers.len(), 1);

    if let Layer::Conductor(conductor) = &stack.layers[0] {
        assert_eq!(conductor.name, "M8");
        assert_relative_eq!(conductor.thickness, 0.885450, epsilon = 1e-6);

        // Check that the CRT_VS_SI_WIDTH table was parsed correctly
        let crt_table = conductor.crt_vs_si_width.as_ref().unwrap();
        assert_eq!(crt_table.widths.len(), 8);
        assert_eq!(crt_table.crt1_values.len(), 8);
        assert_eq!(crt_table.crt2_values.len(), 8);

        // Test exact values
        assert_relative_eq!(crt_table.widths[0], 0.3900, epsilon = 1e-6);
        assert_relative_eq!(crt_table.crt1_values[0], 3.6490e-03, epsilon = 1e-9);
        assert_relative_eq!(crt_table.crt2_values[0], -8.5347e-07, epsilon = 1e-12);

        assert_relative_eq!(crt_table.widths[3], 0.7000, epsilon = 1e-6);
        assert_relative_eq!(crt_table.crt1_values[3], 3.7416e-03, epsilon = 1e-9);
        assert_relative_eq!(crt_table.crt2_values[3], -8.9018e-07, epsilon = 1e-12);

        // Test lookup functionality - exact match
        let (crt1, crt2) = crt_table.lookup_crt_values(0.7000).unwrap();
        assert_relative_eq!(crt1, 3.7416e-03, epsilon = 1e-9);
        assert_relative_eq!(crt2, -8.9018e-07, epsilon = 1e-12);

        // Test interpolation
        let (crt1, crt2) = crt_table.lookup_crt_values(0.45).unwrap();
        // Should interpolate between widths[0] (0.39) and widths[1] (0.4572)
        let t = (0.45 - 0.39) / (0.4572 - 0.39);
        let expected_crt1 = 3.6490e-03 + t * (3.6834e-03 - 3.6490e-03);
        let expected_crt2 = -8.5347e-07 + t * (-8.5317e-07 - (-8.5347e-07));
        assert_relative_eq!(crt1, expected_crt1, epsilon = 1e-9);
        assert_relative_eq!(crt2, expected_crt2, epsilon = 1e-12);

        // Test boundary conditions
        let (crt1, crt2) = crt_table.lookup_crt_values(0.2).unwrap(); // Below range
        assert_relative_eq!(crt1, 3.6490e-03, epsilon = 1e-9);
        assert_relative_eq!(crt2, -8.5347e-07, epsilon = 1e-12);

        let (crt1, crt2) = crt_table.lookup_crt_values(10.0).unwrap(); // Above range
        assert_relative_eq!(crt1, 3.8055e-03, epsilon = 1e-9);
        assert_relative_eq!(crt2, -4.7080e-07, epsilon = 1e-12);
    } else {
        panic!("Expected conductor layer");
    }
}

#[test]
fn test_rho_vs_si_width_thickness_parsing() {
    let itf_content = r#"
TECHNOLOGY = test_tech
GLOBAL_TEMPERATURE = 25.0

CONDUCTOR M8 {
    THICKNESS = 0.885450
    WMIN = 0.32
    SMIN = 0.36
    RHO_VS_SI_WIDTH_AND_THICKNESS {
        WIDTH { 0.3240 0.3600 0.3960 0.4320 }
        THICKNESS { 0.5950 0.6800 0.7650 }
        VALUES {
            0.0206 0.0204 0.0202 0.0201
            0.0205 0.0203 0.0202 0.0200
            0.0205 0.0203 0.0201 0.0200
        }
    }
}
"#;

    let result = parse_itf_file(itf_content);
    assert!(result.is_ok());

    let stack = result.unwrap();
    assert_eq!(stack.layers.len(), 1);

    if let Layer::Conductor(conductor) = &stack.layers[0] {
        assert_eq!(conductor.name, "M8");

        // Check that the RHO_VS_SI_WIDTH_AND_THICKNESS table was parsed correctly
        let rho_table = conductor.rho_vs_si_width_thickness.as_ref().unwrap();
        assert_eq!(rho_table.widths.len(), 4);
        assert_eq!(rho_table.spacings.len(), 3); // These are thicknesses
        assert_eq!(rho_table.values.len(), 3); // 3 thickness rows
        assert_eq!(rho_table.values[0].len(), 4); // 4 width columns

        // Test exact values
        assert_relative_eq!(rho_table.widths[0], 0.3240, epsilon = 1e-6);
        assert_relative_eq!(rho_table.spacings[0], 0.5950, epsilon = 1e-6); // thickness
        assert_relative_eq!(rho_table.values[0][0], 0.0206, epsilon = 1e-6);
        assert_relative_eq!(rho_table.values[2][3], 0.0200, epsilon = 1e-6);

        // Test lookup functionality
        let rho = rho_table.lookup(0.3240, 0.5950).unwrap();
        assert_relative_eq!(rho, 0.0206, epsilon = 1e-6);
    } else {
        panic!("Expected conductor layer");
    }
}

#[test]
fn test_resistance_calculation_with_crt_table() {
    let itf_content = r#"
TECHNOLOGY = test_tech
GLOBAL_TEMPERATURE = 25.0

CONDUCTOR M1 {
    THICKNESS = 0.2
    RPSQ = 0.05
    CRT_VS_SI_WIDTH {
        (0.5, 0.003, -1e-7)
        (1.0, 0.0035, -1.2e-7)
        (2.0, 0.004, -1.5e-7)
    }
    WMIN = 0.5
    SMIN = 0.5
}
"#;

    let result = parse_itf_file(itf_content);
    assert!(result.is_ok());

    let stack = result.unwrap();
    if let Layer::Conductor(conductor) = &stack.layers[0] {
        // Test resistance calculation with width-dependent CRT values
        let resistance = conductor.calculate_resistance(1.0, 10.0, 75.0, 25.0);
        assert!(resistance.is_some());

        let r = resistance.unwrap();
        assert!(r > 0.0);

        // Test that CRT values are being looked up correctly
        let crt_table = conductor.crt_vs_si_width.as_ref().unwrap();
        let (crt1, crt2) = crt_table.lookup_crt_values(1.0).unwrap();
        assert_relative_eq!(crt1, 0.0035, epsilon = 1e-6);
        assert_relative_eq!(crt2, -1.2e-7, epsilon = 1e-12);

        // Test interpolation for width between table entries
        let (crt1, crt2) = crt_table.lookup_crt_values(1.5).unwrap();
        let expected_crt1 = 0.0035 + 0.5 * (0.004 - 0.0035); // Linear interpolation
        let expected_crt2 = -1.2e-7 + 0.5 * (-1.5e-7 - (-1.2e-7));
        assert_relative_eq!(crt1, expected_crt1, epsilon = 1e-6);
        assert_relative_eq!(crt2, expected_crt2, epsilon = 1e-12);
    } else {
        panic!("Expected conductor layer");
    }
}

#[test]
fn test_complex_itf_with_all_tables() {
    let itf_content = r#"
TECHNOLOGY = test_tech
GLOBAL_TEMPERATURE = 25.0

DIELECTRIC IMD1 { THICKNESS = 0.1 ER = 4.2 }

CONDUCTOR M1 {
    THICKNESS = 0.5
    WMIN = 0.3
    SMIN = 0.3
    CRT_VS_SI_WIDTH {
        (0.3, 0.003, -1e-7)
        (0.5, 0.0032, -1.1e-7)
        (1.0, 0.0035, -1.2e-7)
    }
    RHO_VS_SI_WIDTH_AND_THICKNESS {
        WIDTH { 0.3 0.5 1.0 }
        THICKNESS { 0.4 0.5 0.6 }
        VALUES {
            0.025 0.024 0.023
            0.024 0.023 0.022
            0.023 0.022 0.021
        }
    }
    ETCH_VS_WIDTH_AND_SPACING {
        WIDTHS { 0.3 0.5 1.0 }
        SPACINGS { 0.3 0.5 1.0 }
        VALUES {
            0.01 0.015 0.02
            0.008 0.012 0.018
            0.005 0.008 0.015
        }
    }
}
"#;

    let result = parse_itf_file(itf_content);
    assert!(result.is_ok());

    let stack = result.unwrap();
    assert_eq!(stack.layers.len(), 2); // 1 dielectric + 1 conductor

    // Check that all tables were parsed
    if let Layer::Conductor(conductor) = &stack.layers[1] {
        assert!(conductor.crt_vs_si_width.is_some());
        assert!(conductor.rho_vs_si_width_thickness.is_some());
        assert!(conductor.etch_vs_width_spacing.is_some());

        let crt_table = conductor.crt_vs_si_width.as_ref().unwrap();
        assert_eq!(crt_table.widths.len(), 3);

        let rho_table = conductor.rho_vs_si_width_thickness.as_ref().unwrap();
        assert_eq!(rho_table.widths.len(), 3);
        assert_eq!(rho_table.spacings.len(), 3);

        let etch_table = conductor.etch_vs_width_spacing.as_ref().unwrap();
        assert_eq!(etch_table.widths.len(), 3);
        assert_eq!(etch_table.spacings.len(), 3);
    } else {
        panic!("Expected conductor layer at index 1");
    }
}
