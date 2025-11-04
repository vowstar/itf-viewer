// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use crate::data::ProcessStack;
use crate::parser::parse_itf_file;
use egui::{Context, RichText, Window};
use poll_promise::Promise;
use rfd::AsyncFileDialog;
use std::path::PathBuf;

pub struct FileMenu {
    pub is_open: bool,
    pub selected_file: Option<PathBuf>,
    pub error_message: Option<String>,
    pub load_result: Option<Result<ProcessStack, String>>,
    file_dialog_promise: Option<Promise<Option<PathBuf>>>,
}

impl FileMenu {
    pub fn new() -> Self {
        Self {
            is_open: false,
            selected_file: None,
            error_message: None,
            load_result: None,
            file_dialog_promise: None,
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        // Check if file dialog promise is ready
        if let Some(promise) = &self.file_dialog_promise {
            if let Some(result) = promise.ready() {
                if let Some(path) = result {
                    self.selected_file = Some(path.clone());
                    self.load_file(path.clone());
                }
                self.file_dialog_promise = None;
            }
        }

        if self.is_open {
            Window::new("File Operations")
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Open ITF File").clicked() {
                            self.open_native_file_dialog();
                        }

                        ui.separator();

                        if ui.button("Close").clicked() {
                            self.is_open = false;
                        }
                    });

                    ui.separator();

                    // Show selected file path if any
                    if let Some(ref path) = self.selected_file {
                        ui.horizontal(|ui| {
                            ui.label("File:");
                            ui.label(
                                path.file_name()
                                    .map(|name| name.to_string_lossy())
                                    .unwrap_or_else(|| "Unknown".into())
                                    .to_string(),
                            );
                        });

                        ui.horizontal(|ui| {
                            if ui.button("Clear Selection").clicked() {
                                self.selected_file = None;
                                self.error_message = None;
                                self.load_result = None;
                            }
                        });
                    }

                    // Show error message if any
                    if let Some(ref error) = self.error_message {
                        ui.separator();
                        ui.colored_label(egui::Color32::RED, format!("Error: {error}"));
                    }

                    // Show load result summary
                    if let Some(Ok(ref stack)) = self.load_result {
                        ui.separator();
                        ui.label(
                            RichText::new("File loaded successfully!").color(egui::Color32::GREEN),
                        );

                        let summary = stack.get_process_summary();
                        ui.group(|ui| {
                            ui.label(format!("Technology: {}", summary.technology_name));
                            ui.label(format!("Total layers: {}", summary.total_layers));
                            ui.label(format!("Conductors: {}", summary.conductor_layers));
                            ui.label(format!("Dielectrics: {}", summary.dielectric_layers));
                            ui.label(format!("Via connections: {}", summary.via_connections));
                            if let Some(temp) = summary.global_temperature {
                                ui.label(format!("Temperature: {temp:.1}°C"));
                            }
                            ui.label(format!("Total height: {:.3} um", summary.total_height));
                        });
                    }
                });
        }
    }

    fn open_native_file_dialog(&mut self) {
        // Use async file dialog to avoid blocking the UI thread
        let task = AsyncFileDialog::new()
            .add_filter("ITF Files", &["itf"])
            .add_filter("All Files", &["*"])
            .set_title("Select ITF File")
            .pick_file();

        // Create a promise that resolves to the selected file path
        // Use spawn_thread to run the async task in a background thread
        let promise = Promise::spawn_thread("file_dialog", move || {
            // Block on the async task in the background thread
            pollster::block_on(async move { task.await.map(|handle| handle.path().to_path_buf()) })
        });

        self.file_dialog_promise = Some(promise);
    }

    fn load_file(&mut self, path: PathBuf) {
        match std::fs::read_to_string(&path) {
            Ok(content) => match parse_itf_file(&content) {
                Ok(stack) => {
                    self.load_result = Some(Ok(stack));
                    self.error_message = None;
                }
                Err(e) => {
                    self.error_message = Some(format!("Parse error: {e}"));
                    self.load_result = None;
                }
            },
            Err(e) => {
                self.error_message = Some(format!("File read error: {e}"));
                self.load_result = None;
            }
        }
    }

    pub fn get_loaded_stack(&self) -> Option<&ProcessStack> {
        if let Some(Ok(ref stack)) = self.load_result {
            Some(stack)
        } else {
            None
        }
    }

    pub fn take_loaded_stack(&mut self) -> Option<ProcessStack> {
        if let Some(Ok(stack)) = self.load_result.take() {
            Some(stack)
        } else {
            None
        }
    }

    pub fn clear_load_result(&mut self) {
        self.load_result = None;
        self.error_message = None;
    }

    pub fn has_loaded_stack(&self) -> bool {
        matches!(self.load_result, Some(Ok(_)))
    }
}

impl Default for FileMenu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_menu_creation() {
        let menu = FileMenu::new();
        assert!(!menu.is_open);
        assert!(menu.selected_file.is_none());
        assert!(menu.error_message.is_none());
        assert!(menu.load_result.is_none());
    }

    #[test]
    fn test_file_selection() {
        let mut menu = FileMenu::new();

        // Simulate file selection
        menu.selected_file = Some(PathBuf::from("test.itf"));
        assert!(menu.selected_file.is_some());

        // Clear selection
        menu.selected_file = None;
        menu.error_message = None;
        menu.load_result = None;

        assert!(menu.selected_file.is_none());
        assert!(menu.error_message.is_none());
        assert!(menu.load_result.is_none());
    }

    #[test]
    fn test_load_result_handling() {
        let mut menu = FileMenu::new();

        // No loaded stack initially
        assert!(!menu.has_loaded_stack());
        assert!(menu.get_loaded_stack().is_none());

        // Simulate successful load (would need actual test data)
        // This test is more for API validation than actual file loading
        assert!(menu.take_loaded_stack().is_none());
    }

    #[test]
    fn test_error_handling() {
        let mut menu = FileMenu::new();

        // Set error message
        menu.error_message = Some("Test error".to_string());
        assert!(menu.error_message.is_some());

        // Clear error
        menu.clear_load_result();
        assert!(menu.error_message.is_none());
    }
}
