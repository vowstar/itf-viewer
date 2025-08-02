// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use itf_viewer::data::*;
use itf_viewer::parser::parse_itf_file;
use std::fs;

#[test]
fn test_parse_complex_itf_file() {
    let itf_path = "tests/data/complex_test.itf";

    // Read the complex test ITF file
    let content = fs::read_to_string(itf_path).expect("Failed to read ITF file");

    let result = parse_itf_file(&content);
    assert!(
        result.is_ok(),
        "Failed to parse ITF file: {:?}",
        result.err()
    );

    let stack = result.unwrap();

    // Check technology info - adjust based on the actual test file
    println!("Technology name: {}", stack.technology_info.name);
    println!(
        "Global temperature: {:?}",
        stack.technology_info.global_temperature
    );

    println!("Total layers: {}", stack.layers.len());
    println!("Total vias: {}", stack.via_stack.vias.len());

    // Find the M1 conductor layer which should have CRT_VS_SI_WIDTH
    let m1_layer = stack.layers.iter().find(|layer| layer.name() == "M1");

    if let Some(Layer::Conductor(conductor)) = m1_layer {
        println!("Found M1 conductor layer");

        // Check if CRT_VS_SI_WIDTH was parsed
        if let Some(crt_table) = &conductor.crt_vs_si_width {
            println!(
                "CRT_VS_SI_WIDTH table found with {} entries",
                crt_table.widths.len()
            );
            assert!(crt_table.widths.len() > 0);
            assert_eq!(crt_table.widths.len(), crt_table.crt1_values.len());
            assert_eq!(crt_table.widths.len(), crt_table.crt2_values.len());

            // Test some values from the actual file
            // (0.3900, 3.6490e-03, -8.5347e-07) should be the first entry
            if crt_table.widths.len() > 0 {
                println!(
                    "First CRT entry: width={}, crt1={}, crt2={}",
                    crt_table.widths[0], crt_table.crt1_values[0], crt_table.crt2_values[0]
                );
            }
        } else {
            println!("CRT_VS_SI_WIDTH table not found in M1");
        }

        // Check if RHO_VS_SI_WIDTH_AND_THICKNESS was parsed
        if let Some(rho_table) = &conductor.rho_vs_si_width_thickness {
            println!("RHO_VS_SI_WIDTH_AND_THICKNESS table found");
            println!("Widths: {} entries", rho_table.widths.len());
            println!("Thicknesses: {} entries", rho_table.spacings.len());
            println!(
                "Values: {}x{} matrix",
                rho_table.values.len(),
                rho_table.values.get(0).map(|v| v.len()).unwrap_or(0)
            );
        } else {
            println!("RHO_VS_SI_WIDTH_AND_THICKNESS table not found in M1");
        }

        // Check if ETCH_VS_WIDTH_AND_SPACING was parsed
        if let Some(etch_table) = &conductor.etch_vs_width_spacing {
            println!("ETCH_VS_WIDTH_AND_SPACING table found");
            println!("Widths: {} entries", etch_table.widths.len());
            println!("Spacings: {} entries", etch_table.spacings.len());
            println!(
                "Values: {}x{} matrix",
                etch_table.values.len(),
                etch_table.values.get(0).map(|v| v.len()).unwrap_or(0)
            );
        } else {
            println!("ETCH_VS_WIDTH_AND_SPACING table not found in M1");
        }
    } else {
        panic!("M1 conductor layer not found");
    }
}
