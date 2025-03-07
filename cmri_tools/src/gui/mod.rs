//! Shared behaviour for GUIs.

use std::{collections::HashMap, ops::{BitAnd, BitXorAssign}};

pub mod connection;
pub mod menu;

/// A reference which can be mutable or not.
/// Used to control whether the item is editable when displayed.
#[derive(Debug)]
pub enum MaybeMutable<'a, T> where T: std::fmt::Debug {
    /// The item pointed to is readonly - that is don't allow editing when displayed.
    ReadOnly(&'a T),
    /// The item pointed to is mutable - that is allow editing when displayed.
    Mutable(&'a mut T)
}

impl<T> MaybeMutable<'_, T> where T: std::fmt::Debug {
    const fn is_mut(&self) -> bool {
        matches!(self, Mutable(_))
    }

    fn map<U>(&mut self, mutable: impl FnOnce(&mut T) -> &mut U, read_only: impl FnOnce(&T) -> &U) -> MaybeMutable<U> where U: std::fmt::Debug {
        match self {
            Mutable(t) => Mutable(mutable(*t)),
            ReadOnly(t) => ReadOnly(read_only(*t))
        }
    }

    // fn either_or<U>(&mut self, mutable: impl FnOnce(&mut T) -> U, read_only: impl FnOnce(&T) -> U) -> U {
    //     match self {
    //         Mutable(t) => mutable(*t),
    //         ReadOnly(t) => read_only(*t)
    //     }
    // }

    // fn if_mutable<U>(&mut self, action: impl FnOnce(&mut T) -> U) -> Option<U> {
    //     if let Mutable(t) = self {
    //         Some(action(*t))
    //     } else {
    //         None
    //     }
    // }

    // fn if_read_only<U>(&self, action: impl FnOnce(&T) -> U) -> Option<U> {
    //     if let ReadOnly(t) = self {
    //         Some(action(*t))
    //     } else {
    //         None
    //     }
    // }
}

impl<T> AsRef<T> for MaybeMutable<'_, T> where T: std::fmt::Debug {
    fn as_ref(&self) -> &T {
        match self {
            ReadOnly(t) => t,
            Mutable(t) => t
        }
    }
}

impl<T> AsMut<T> for MaybeMutable<'_, T> where T: std::fmt::Debug {
    fn as_mut(&mut self) -> &mut T {
        match self {
            ReadOnly(t) => panic!("Reference to {t:?} is not mutable."),
            Mutable(t) => t
        }
    }
}

pub use MaybeMutable::{ReadOnly, Mutable};


/// Use an `egui_modal::Modal` to prompt the user for information, sticking with consistent dialog styling.
pub fn modal_prompt(modal: &egui_modal::Modal, title: impl Into<egui::RichText>, content: impl FnOnce(&mut egui::Ui), buttons: impl FnOnce(&egui_modal::Modal, &mut egui::Ui)) {
    modal.show(|ui| {
        modal.title(ui, title);
        modal.frame(ui, |ui| {
            egui::Grid::new("modal_prompt")
                .num_columns(2)
                .show(ui, |ui| {
                    modal.icon(ui, egui_modal::Icon::Custom((String::from("?"), egui::Color32::from_rgb(150, 200, 210))));
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), content);
                });
        });
        modal.buttons(ui, |ui| buttons(modal, ui));
    });
    modal.open();
}


/// Use an `egui_modal::Modal` to display an error, sticking with consistent dialog styling.
pub fn modal_error(modal: &egui_modal::Modal, error: &anyhow::Error) {
    let mut details = String::new();
    error.chain().skip(1).for_each(|cause| details.push_str(&format!("because: {cause}\n")));
    modal.dialog()
        .with_title(error.to_string())
        .with_body(details.trim())
        .with_icon(egui_modal::Icon::Error)
        .open();
}


/// Prompt the user for a file.
#[expect(clippy::missing_panics_doc)]
pub fn file_prompt(title: impl Into<String>, file_path: Option<&std::path::PathBuf>) -> rfd::FileDialog {
    let directory = file_path.as_ref().map_or_else(
        || dirs::document_dir().expect("Couldn't workout your Documents directory."),
        |path| path.parent().expect("File to have a directory.").to_path_buf()
    );
    let file_name = file_path.as_ref()
        .and_then(|file| file.file_name().expect("Always has a filename element").to_str())
        .unwrap_or("my_cmri_network.json");
    rfd::FileDialog::new()
        .set_title(title)
        .set_directory(directory)
        .set_file_name(file_name)
        .add_filter("JSON", &["json"])
}


/// List the bits from a `cmri::packet::Data` (using the labels) as a vertical list of `egui::SelectableLabel`s.
///
/// If ditable is true clicking the label will toggle the underlying bit.
pub fn list_of_bits<H: std::hash::BuildHasher>(ui: &mut egui::Ui, mut data: MaybeMutable<cmri::packet::Data>, labels: &HashMap<usize, String, H>) {
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        for i in 0..(data.as_ref().len() * 8) {
            if i % 8 == 0 {
                if i != 0 {
                    ui.separator();
                }
                ui.label(format!("Byte {}", i / 8));
            }
            let label = labels.get(&i).map_or_else(
                || format!("Bit {} ({})", i % 8, i),
                |l| format!("Bit {} ({}): {}", i % 8, i, l)
            );
            if data.is_mut() {
                if ui.selectable_label(data.as_mut().get_bit(i), label).clicked() {
                    data.as_mut().toggle_bit(i);
                }
            } else {
                let _ = ui.selectable_label(data.as_ref().get_bit(i), label);
            }
        }
    });
}


/// Show bytes in a scrollable grid, whilst allowing them to be changed.
pub fn list_of_bytes<H: std::hash::BuildHasher>(ui: &mut egui::Ui, per_row: u8, heading: &str, mut data: MaybeMutable<cmri::packet::Data>, labels: &HashMap<usize, String, H>) {
    let count = data.as_ref().len();
    let heading_height = ui.style().text_styles.get(&egui::style::TextStyle::Heading)
        .map_or(18.0, |s| s.size);
    let per_row = per_row as usize;
    let rows = count.div_ceil(per_row);
    let bit_size = ui.spacing().interact_size.y * 1.75;
    let byte_size = bit_size * egui::vec2(4.0, 1.5);
    let byte_spacing = egui::Vec2 { x: 12.0, y: 12.0 };
    let row_and_spacing = byte_size.y + byte_spacing.y;
    #[expect(clippy::cast_precision_loss)]
    let height = row_and_spacing.mul_add(rows as f32, -byte_spacing.y) + heading_height;
    let col_and_spacing = byte_size.x + byte_spacing.x;
    #[expect(clippy::cast_precision_loss)]
    let width = col_and_spacing.mul_add(per_row as f32, -byte_spacing.x);

    let (rect, mut _response) = ui.allocate_exact_size(egui::Vec2 { x: width, y: height }, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let (heading_rect, bytes_rect) = rect.split_top_bottom_at_y(rect.min.y + heading_height);
        ui.new_child(egui::UiBuilder::new().max_rect(heading_rect))
            .heading(heading);

        let mut min = bytes_rect.min;
        for row in 0..rows {
            min.x = bytes_rect.min.x;
            for i in 0..per_row {
                let index = (row * per_row) + i;
                if index < count {
                    let rect = egui::Rect { min, max: min + byte_size };
                    let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                    byte(&mut ui, bit_size, byte_size, index, data.map(|a| &mut a[index], |a| &a[index]), labels);
                }
                min.x += col_and_spacing;
            }
            min.y += row_and_spacing;
        }
    }
}

fn byte<H: std::hash::BuildHasher>(ui: &mut egui::Ui, bit_size: f32, byte_size: egui::Vec2, index: usize, mut value: MaybeMutable<u8>, labels: &HashMap<usize, String, H>) {
    let (rect, mut response) = ui.allocate_exact_size(byte_size, egui::Sense::click());
    let (top_rect, bit_rects) = byte_rects(&rect, bit_size);

    if let Some(pos) = response.hover_pos() {
        let bit = bit_rects
            .iter()
            .enumerate()
            .find(|(_i, r)| r.contains(pos) )
            .map(|(i, _r)| i);
        if let Some(bit) = bit {
            let bit_index = index + bit;
            let text = labels.get(&bit_index).map_or_else(
                || format!("bit {bit} {}", if value.as_ref().bitand(1 << bit) > 0 { "on" } else { "off" }),
                |label| format!("bit {bit} ({label}) {}", if value.as_ref().bitand(1 << bit) > 0 { "on" } else { "off" })
            );
            response = response.on_hover_text_at_pointer(text);
        };
    }

    if value.is_mut() && response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let bit = bit_rects
                .iter()
                .enumerate()
                .find(|(_i, r)| r.contains(pos) )
                .map(|(i, _r)| i);
            if let Some(bit) = bit {
                value.as_mut().bitxor_assign(1 << bit);
                response.mark_changed();
            };
        }
    }

    if ui.is_rect_visible(rect) {
        ui.new_child(egui::UiBuilder::new().max_rect(top_rect))
            .label(format!("Byte\u{00A0}{index:3}"))
            .on_hover_text_at_pointer(format!("{value} 0x{value:02X} 0b{value:08b}", value = value.as_ref()));

        #[expect(clippy::needless_range_loop)]
        for i in 0..8 {
            let color = if (value.as_ref().bitand(1 << i)) > 0 { egui::Color32::LIGHT_BLUE } else { egui::Color32::DARK_GRAY };
            ui.painter().rect(bit_rects[i], 0.0, color, egui::Stroke::NONE);
        }
    }
}

fn byte_rects(rect: &egui::Rect, bit_size: f32) -> (egui::Rect, [egui::Rect; 8]) {
    let (top_rect, bottom_rect) = rect.split_top_bottom_at_y(rect.y_range().min + bit_size);
    let y_min = bottom_rect.y_range().min;
    let y_max = bottom_rect.y_range().max;
    let x_min = bottom_rect.x_range().min;
    let x_max = bottom_rect.x_range().max;
    let x_delta = bottom_rect.x_range().span() / 31.0;
    let bit_rects = [
        egui::Rect { min: egui::Pos2 { x: x_min, y: y_min}, max: egui::Pos2 { x: x_delta.mul_add(3.0, x_min), y: y_max} },
        egui::Rect { min: egui::Pos2 { x: x_delta.mul_add(4.0, x_min), y: y_min}, max: egui::Pos2 { x: x_delta.mul_add(7.0, x_min), y: y_max} },
        egui::Rect { min: egui::Pos2 { x: x_delta.mul_add(8.0, x_min), y: y_min}, max: egui::Pos2 { x: x_delta.mul_add(11.0, x_min), y: y_max} },
        egui::Rect { min: egui::Pos2 { x: x_delta.mul_add(12.0, x_min), y: y_min}, max: egui::Pos2 { x: x_delta.mul_add(15.0, x_min), y: y_max} },
        egui::Rect { min: egui::Pos2 { x: x_delta.mul_add(16.0, x_min), y: y_min}, max: egui::Pos2 { x: x_delta.mul_add(19.0, x_min), y: y_max} },
        egui::Rect { min: egui::Pos2 { x: x_delta.mul_add(20.0, x_min), y: y_min}, max: egui::Pos2 { x: x_delta.mul_add(23.0, x_min), y: y_max} },
        egui::Rect { min: egui::Pos2 { x: x_delta.mul_add(24.0, x_min), y: y_min}, max: egui::Pos2 { x: x_delta.mul_add(27.0, x_min), y: y_max} },
        egui::Rect { min: egui::Pos2 { x: x_delta.mul_add(28.0, x_min), y: y_min}, max: egui::Pos2 { x: x_max, y: y_max} },
    ];
    (top_rect, bit_rects)
}
