# ITF Viewer

A cross-platform GUI application for viewing and analyzing ITF (Interconnect Technology Format) files used in semiconductor process design.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![CI](https://github.com/vowstar/itf-viewer/actions/workflows/ci.yml/badge.svg)](https://github.com/vowstar/itf-viewer/actions/workflows/ci.yml)

## Overview

ITF Viewer visualizes semiconductor process stacks defined in ITF (Interconnect Technology Format) files. ITF files contain layer structures, electrical properties, and interconnect parameters used in semiconductor design.

## Features

- ITF file parsing with error reporting
- Cross-sectional stack visualization with pan, zoom, and layer selection
- Color-coded display for conductor and dielectric layers
- Layer property inspector (electrical, physical, lookup tables)
- Trapezoid rendering for metal etch profiles
- Via connection visualization
- Thickness exaggeration mode for thin layers

## Supported ITF Features

- Conductor and dielectric layers
- Electrical properties (CRT1/CRT2, RPSQ, RPV)
- Physical properties (width/spacing constraints, side tangent, etch parameters)
- Width/spacing dependent lookup tables
- Via definitions with resistance values
- Technology parameters (temperature, reference direction)

## Installation

### Prerequisites

- Rust 1.70 or later
- Cargo package manager

### From Source

```bash
git clone https://github.com/vowstar/itf-viewer.git
cd itf-viewer
cargo build --release
```

The executable will be available at `target/release/itf-viewer`.

## Usage

Launch the application to open the interactive GUI:

```bash
# Open with file picker
./itf-viewer

# Open with specific ITF file
./itf-viewer path/to/file.itf

# Display help information
./itf-viewer --help

# Show version information
./itf-viewer --version
```

### GUI Controls

- **File Menu**: Open ITF files and settings
- **Layer Panel**: Layer list with selection controls
- **Details Panel**: Property inspector for selected layers
- **Mouse Controls**: Pan (drag), zoom (wheel), select (click)

### View Options

- Toggle layer dimensions, names, and property displays
- Schematic mode with thickness exaggeration

## ITF Format Support

ITF Viewer supports standard ITF format:

### Basic Structure

```itf
TECHNOLOGY = technology_name
GLOBAL_TEMPERATURE = 25.0
REFERENCE_DIRECTION = VERTICAL
BACKGROUND_ER = 4.0

DIELECTRIC layer_name {THICKNESS=1.0 ER=4.2}
CONDUCTOR layer_name {THICKNESS=0.5 RPSQ=0.1}
VIA via_name {FROM=layer1 TO=layer2 AREA=0.01 RPV=1.0}
```

### Advanced Features

- Width/spacing lookup tables
- Temperature coefficients
- Etch parameters
- Multiple metal layers

## Examples

Sample ITF files are in the `examples/` directory.

## Library Usage

ITF Viewer can also be used as a Rust library:

```rust
use itf_viewer::parse_itf_from_file;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stack = parse_itf_from_file("example.itf")?;
    
    let summary = stack.get_process_summary();
    println!("Technology: {}", summary.technology_name);
    println!("Total layers: {}", summary.total_layers);
    println!("Stack height: {:.3} μm", summary.total_height);
    
    Ok(())
}
```

## Architecture

- **`data`**: Core data structures
- **`parser`**: ITF file parsing
- **`renderer`**: Visualization engine
- **`gui`**: User interface (egui)
- **`utils`**: File I/O utilities

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Running with Debug Output

```bash
RUST_LOG=debug cargo run -- example.itf
```

### Code Quality

Includes unit tests, integration tests, and parser validation.

## Dependencies

- egui: GUI framework
- eframe: Application framework
- nom: Parser combinators
- serde: Serialization
- anyhow: Error handling
- rfd: File dialogs

## Platform Support

Linux (X11/Wayland), Windows, macOS

## Contributing

Pull requests and issues are welcome.

### Guidelines

1. Follow Rust conventions
2. Add tests for new features
3. Update documentation as needed

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- ITF format specification
- egui framework

## Related Projects

- StarRC: RC extraction tool
- Calibre: DRC/LVS tool with ITF support
- OpenRCX: Open-source RC extraction

## Contact

- Author: Huang Rui <vowstar@gmail.com>
- Repository: <https://github.com/vowstar/itf-viewer>
- Issues: <https://github.com/vowstar/itf-viewer/issues>
