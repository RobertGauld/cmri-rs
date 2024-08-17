//! Shared behaviour for GUIs related to menus.

use super::{modal_error, file_prompt};

/// Behaviour commonly found within the File menu.
pub mod file {
    use anyhow::Context;
    use super::{modal_error, file_prompt};

    /// Button to exit the application by calling `std::process::exit`.
    pub fn exit(ui: &mut egui::Ui) {
        if ui.button("Exit").clicked() {
            std::process::exit(0);
        }
    }

    /// Button to load nodes from a file.
    ///
    /// Opens the file picker in path, and updates it once a file is picked.
    /// Makes use of modal for displaying any errors which occured.
    /// Runs action, passing it the loaded nodes.
    pub fn load_nodes(ui: &mut egui::Ui, modal: &egui_modal::Modal, file_path: &mut Option<std::path::PathBuf>) -> Option<Vec<Option<crate::file::Node>>> {
        if ui.button("Load Nodes").clicked() {
            if let Some(file) = file_prompt("Load Nodes", file_path.as_ref()).pick_file() {
                match crate::file::load_nodes(file.as_path()).context("Failed to load network.") {
                    Err(error) => modal_error(modal, &error),
                    Ok(nodes) => {
                        file_path.replace(file);
                        return Some(nodes);
                    }
                }
            }
        }
        None
    }

    /// Button to save nodes from a file.
    ///
    /// Opens the file picker in path, and updates it once a file is picked.
    /// Makes use of modal for displaying any errors which occured.
    pub fn save_nodes(ui: &mut egui::Ui, modal: &egui_modal::Modal, file_path: &mut Option<std::path::PathBuf>, get: impl Fn() -> Vec<crate::file::Node>) {
        if ui.button("Save Nodes").clicked() {
            if let Some(file) = file_prompt("Save Nodes", file_path.as_ref()).pick_file() {
                match crate::file::save_nodes(&file, get()).context("Failed to save network.") {
                    Err(error) => modal_error(modal, &error),
                    Ok(()) => { file_path.replace(file); }
                }
            }
        }
    }
}
