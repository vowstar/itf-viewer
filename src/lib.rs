// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

//! ITF Viewer Library
//!
//! A Rust library for parsing and visualizing ITF (Interconnect Technology Format) files
//! used in semiconductor process technology definitions.
//!
//! # Features
//!
//! - Parse ITF files with comprehensive error handling
//! - Represent process stacks with layers, vias, and electrical properties
//! - Render cross-sectional visualizations with proper color coding
//! - Interactive GUI with pan, zoom, and layer selection
//! - Support for trapezoid shapes representing etched/deposited metals
//!
//! # Usage
//!
//! ```rust,no_run
//! use itf_viewer::parser::parse_itf_file;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Parse an ITF file
//! let content = std::fs::read_to_string("example.itf")?;
//! let stack = parse_itf_file(&content)?;
//!
//! // Get process summary
//! let summary = stack.get_process_summary();
//! println!("Technology: {}", summary.technology_name);
//! println!("Total layers: {}", summary.total_layers);
//! # Ok(())
//! # }
//! ```
//!
//! # Architecture
//!
//! The library is organized into several modules:
//!
//! - `data`: Core data structures for layers, vias, and process stacks
//! - `parser`: ITF file parsing with lexical analysis and syntax parsing
//! - `renderer`: Visualization rendering with colors and geometry
//! - `gui`: Interactive GUI components using egui framework
//! - `utils`: Utility functions for file I/O and helpers

pub mod data;
pub mod gui;
pub mod parser;
pub mod renderer;
pub mod utils;

#[cfg(test)]
mod integration_tests;

// Re-export commonly used types
pub use data::{
    ConductorLayer, DielectricLayer, Layer, LayerType, ProcessStack, TechnologyInfo, ViaConnection,
    ViaType,
};

pub use parser::{parse_itf_file, ItfParser, ParseError};

pub use renderer::{
    ColorScheme, LayerGeometry, RectangleShape, StackRenderer, TrapezoidShape, ViewTransform,
};

pub use gui::{FileMenu, LayerPanel, MainWindow, StackViewer, Toolbar};

/// Library version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Library description
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

/// Get library information as a formatted string
pub fn get_library_info() -> String {
    format!("{NAME} v{VERSION} - {DESCRIPTION}")
}

/// Parse an ITF file from a file path
///
/// This is a convenience function that reads the file and parses it.
///
/// # Arguments
///
/// * `file_path` - Path to the ITF file
///
/// # Returns
///
/// * `Result<ProcessStack, Box<dyn std::error::Error>>` - The parsed process stack or error
///
/// # Example
///
/// ```rust,no_run
/// use itf_viewer::parse_itf_from_file;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let stack = parse_itf_from_file("example.itf")?;
/// println!("Loaded stack with {} layers", stack.get_layer_count());
/// # Ok(())
/// # }
/// ```
pub fn parse_itf_from_file<P: AsRef<std::path::Path>>(
    file_path: P,
) -> Result<ProcessStack, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(file_path)?;
    let stack = parse_itf_file(&content)?;
    Ok(stack)
}

/// Validate an ITF file without full parsing
///
/// This function performs a quick validation check on an ITF file
/// to determine if it appears to be a valid ITF format.
///
/// # Arguments
///
/// * `content` - The ITF file content as a string
///
/// # Returns
///
/// * `bool` - True if the file appears to be valid ITF format
///
/// # Example
///
/// ```rust,no_run
/// use itf_viewer::validate_itf_content;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let content = std::fs::read_to_string("example.itf")?;
/// if validate_itf_content(&content) {
///     println!("File appears to be valid ITF format");
/// }
/// # Ok(())
/// # }
/// ```
pub fn validate_itf_content(content: &str) -> bool {
    // Basic validation - check for required keywords
    let required_keywords = ["TECHNOLOGY"];
    let content_upper = content.to_uppercase();

    for keyword in &required_keywords {
        if !content_upper.contains(keyword) {
            return false;
        }
    }

    // Check for at least one layer definition
    let layer_keywords = ["DIELECTRIC", "CONDUCTOR"];
    let has_layers = layer_keywords
        .iter()
        .any(|keyword| content_upper.contains(keyword));

    has_layers
}

/// Get default application configuration
///
/// Returns a default configuration suitable for most use cases.
///
/// # Returns
///
/// * `AppConfig` - Default application configuration
pub fn get_default_config() -> AppConfig {
    AppConfig::default()
}

/// Application configuration structure
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Window title
    pub window_title: String,
    /// Initial window width
    pub window_width: f32,
    /// Initial window height  
    pub window_height: f32,
    /// Whether to show dimensions by default
    pub show_dimensions: bool,
    /// Whether to show layer names by default
    pub show_layer_names: bool,
    /// Default layer width for rendering
    pub default_layer_width: f32,
    /// Whether the layer panel is open by default
    pub layer_panel_open: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            window_title: format!("{NAME} v{VERSION}"),
            window_width: 1200.0,
            window_height: 800.0,
            show_dimensions: true,
            show_layer_names: true,
            default_layer_width: 200.0,
            layer_panel_open: true,
        }
    }
}

/// Create and run the ITF viewer application
///
/// This is the main entry point for running the GUI application.
///
/// # Arguments
///
/// * `config` - Application configuration
///
/// # Returns
///
/// * `Result<(), eframe::Error>` - Result of running the application
///
/// # Example
///
/// ```rust,no_run
/// use itf_viewer::{run_app, get_default_config};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config = get_default_config();
/// run_app(config)?;
/// # Ok(())
/// # }
/// ```
pub fn run_app(config: AppConfig) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([config.window_width, config.window_height])
            .with_title(&config.window_title),
        ..Default::default()
    };

    let app = MainWindow::new();

    // Note: Configuration would be applied in the actual app initialization
    // For now, we use defaults in MainWindow::new()

    eframe::run_native(
        &config.window_title,
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_info() {
        let info = get_library_info();
        assert!(info.contains("itf-viewer"));
        assert!(info.contains("0.1.0"));
    }

    #[test]
    fn test_validate_itf_content() {
        // Valid ITF content
        let valid_content = r#"
            TECHNOLOGY = test_tech
            DIELECTRIC oxide {THICKNESS=1.0 ER=4.2}
            CONDUCTOR metal {THICKNESS=0.5}
        "#;
        assert!(validate_itf_content(valid_content));

        // Invalid content - missing TECHNOLOGY
        let invalid_content1 = r#"
            DIELECTRIC oxide {THICKNESS=1.0 ER=4.2}
        "#;
        assert!(!validate_itf_content(invalid_content1));

        // Invalid content - no layers
        let invalid_content2 = r#"
            TECHNOLOGY = test_tech
        "#;
        assert!(!validate_itf_content(invalid_content2));

        // Empty content
        assert!(!validate_itf_content(""));
    }

    #[test]
    fn test_app_config() {
        let config = AppConfig::default();
        assert!(config.window_width > 0.0);
        assert!(config.window_height > 0.0);
        assert!(config.show_dimensions);
        assert!(config.show_layer_names);
        assert!(config.layer_panel_open);
        assert!(config.default_layer_width > 0.0);

        let default_config = get_default_config();
        assert_eq!(config.window_width, default_config.window_width);
        assert_eq!(config.window_height, default_config.window_height);
    }

    #[test]
    fn test_version_constants() {
        assert!(!VERSION.trim().is_empty());
        assert!(!NAME.trim().is_empty());
        assert!(!DESCRIPTION.trim().is_empty());

        assert_eq!(NAME, "itf-viewer");
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn test_config_clone_and_debug() {
        let config = AppConfig::default();
        let cloned = config.clone();

        assert_eq!(config.window_width, cloned.window_width);
        assert_eq!(config.window_height, cloned.window_height);

        // Test Debug formatting (should not panic)
        let debug_str = format!("{config:?}");
        assert!(debug_str.contains("AppConfig"));
        assert!(debug_str.contains("window_title"));
    }
}
