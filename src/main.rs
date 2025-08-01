// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

//! ITF Viewer Application
//!
//! A cross-platform GUI application for viewing and analyzing ITF
//! (Interconnect Technology Format) files used in semiconductor process design.

use itf_viewer::{get_default_config, parse_itf_from_file, run_app};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    match args.len() {
        1 => {
            // No arguments - run GUI application
            run_gui_app()
        }
        2 => {
            // One argument - either file path or command
            let arg = &args[1];
            match arg.as_str() {
                "--help" | "-h" => {
                    print_help();
                    Ok(())
                }
                "--version" | "-v" => {
                    print_version();
                    Ok(())
                }
                _ => {
                    // Assume it's a file path
                    run_with_file(arg)
                }
            }
        }
        _ => {
            eprintln!("Error: Too many arguments");
            print_usage();
            std::process::exit(1);
        }
    }
}

fn run_gui_app() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting ITF Viewer...");

    let config = get_default_config();
    run_app(config).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

fn run_with_file(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Loading ITF file: {file_path}");

    // Validate and parse the file
    match parse_itf_from_file(file_path) {
        Ok(stack) => {
            // Print file information
            print_file_info(&stack);

            // Start GUI with the loaded file
            println!("Starting ITF Viewer with loaded file...");
            let config = get_default_config();
            run_app(config).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        }
        Err(e) => {
            eprintln!("Error loading ITF file: {e}");
            std::process::exit(1);
        }
    }
}

fn print_file_info(stack: &itf_viewer::ProcessStack) {
    let summary = stack.get_process_summary();

    println!("ITF File Information:");
    println!("  Technology: {}", summary.technology_name);
    println!("  Total layers: {}", summary.total_layers);
    println!("  Conductor layers: {}", summary.conductor_layers);
    println!("  Dielectric layers: {}", summary.dielectric_layers);
    println!("  Metal layers: {}", summary.metal_layers);
    println!("  Via connections: {}", summary.via_connections);

    if let Some(temp) = summary.global_temperature {
        println!("  Global temperature: {temp:.1}°C");
    }

    println!("  Total stack height: {:.3} um", summary.total_height);
    println!();
}

fn print_help() {
    println!("{}", itf_viewer::get_library_info());
    println!();
    println!("USAGE:");
    println!("    {} [OPTIONS] [FILE]", env!("CARGO_PKG_NAME"));
    println!();
    println!("ARGS:");
    println!("    <FILE>    ITF file to load and display");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help       Print this help message");
    println!("    -v, --version    Print version information");
    println!();
    println!("DESCRIPTION:");
    println!("    ITF Viewer is a cross-platform application for visualizing semiconductor");
    println!("    process stacks defined in ITF (Interconnect Technology Format) files.");
    println!();
    println!("    The application provides an interactive cross-sectional view of the");
    println!("    process stack, showing layers with proper color coding, dimensions,");
    println!("    and detailed property information.");
    println!();
    println!("FEATURES:");
    println!("    • Parse and validate ITF files");
    println!("    • Interactive pan, zoom, and layer selection");
    println!("    • Color-coded visualization (copper/dielectric materials)");
    println!("    • Trapezoid shapes for etched/deposited metal layers");
    println!("    • Detailed layer property inspection");
    println!("    • Process summary and statistics");
    println!();
    println!("CONTROLS:");
    println!("    • Mouse wheel: Zoom in/out");
    println!("    • Drag: Pan view");
    println!("    • Click: Select layer");
    println!("    • Ctrl+R: Reset view");
    println!("    • Arrow keys: Pan view");
    println!("    • +/- keys: Zoom");
    println!();
    println!("EXAMPLES:");
    println!(
        "    {}                           # Start GUI application",
        env!("CARGO_PKG_NAME")
    );
    println!(
        "    {} process.itf               # Load and display ITF file",
        env!("CARGO_PKG_NAME")
    );
    println!(
        "    {} --version                 # Show version information",
        env!("CARGO_PKG_NAME")
    );
}

fn print_version() {
    println!("{} v{}", itf_viewer::NAME, itf_viewer::VERSION);
    println!("{}", itf_viewer::DESCRIPTION);
    println!();
    println!("Build information:");
    println!(
        "  Profile: {}",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
}

fn print_usage() {
    eprintln!("Usage: {} [OPTIONS] [FILE]", env!("CARGO_PKG_NAME"));
    eprintln!(
        "Try '{} --help' for more information.",
        env!("CARGO_PKG_NAME")
    );
}

// Handle Ctrl+C gracefully
#[allow(dead_code)]
fn setup_signal_handlers() {
    ctrlc::set_handler(move || {
        println!("\nReceived Ctrl+C, shutting down gracefully...");
        std::process::exit(0);
    })
    .expect("Error setting Ctrl+C handler");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_function_with_help() {
        // Test that help doesn't panic
        // In a real test, you might capture stdout and verify content
        print_help();
        print_version();
        print_usage();
    }

    #[test]
    fn test_print_file_info() {
        use itf_viewer::data::{DielectricLayer, Layer, ProcessStack, TechnologyInfo};

        let tech = TechnologyInfo::new("test_tech".to_string());
        let mut stack = ProcessStack::new(tech);
        stack.add_layer(Layer::Dielectric(DielectricLayer::new(
            "oxide".to_string(),
            1.0,
            4.2,
        )));

        // Should not panic
        print_file_info(&stack);
    }
}
