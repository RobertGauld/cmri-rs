use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{trace, warn};
use cmri_tools::{file, gui};
use crate::state::{State, Node, run_connection};

pub const APP_TITLE: &str = "CMRInet Nodes";

#[expect(clippy::unwrap_used, clippy::missing_panics_doc)]
pub fn run(cli_args: &clap::ArgMatches, tokio_handle: tokio::runtime::Handle) {
    let mut show_nodes = [false; 128];
    if let Some(addresses) = cli_args.get_many::<u8>("open-node") {
        for address in addresses {
            let address = usize::from(*address);
            if address <= 127 {
                show_nodes[address] = true;
            } else {
                eprintln!("{address} is an invalid node address.");
                warn!("{address} passed from the command line is an invalid node address.");
            }
        }
    }
    let connection_state = tokio_handle.block_on(async { gui::connection::State::new(cli_args) });
    let file_path = cli_args.get_one::<std::path::PathBuf>("load-nodes").cloned();
    let state = Arc::new(Mutex::new(State::default()));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 600.0]),
        ..eframe::NativeOptions::default()
    };

    trace!("Running egui app.");
    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(move |cc| {
            state.blocking_lock().egui_ctx = cc.egui_ctx.clone();
            let modal = {
                let style = egui_modal::ModalStyle { default_width: Some(500.0), ..Default::default() };
                egui_modal::Modal::new(&cc.egui_ctx, "Modal").with_style(&style)
            };
            Ok(Box::new(App {
                title: String::from(APP_TITLE),
                state,
                show_nodes,
                connection_state,
                file_path,
                modal,
                tokio_handle
            }))
        })
    ).unwrap();
}

struct App {
    title: String,
    state: Arc<Mutex<State>>,
    show_nodes: [bool; 128],
    connection_state: gui::connection::State,
    file_path: Option<std::path::PathBuf>,
    modal: egui_modal::Modal,
    tokio_handle: tokio::runtime::Handle
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.modal.show_dialog();

        egui::TopBottomPanel::top("MenuPanel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if let Some(nodes) = gui::menu::file::load_nodes(ui, &self.modal, &mut self.file_path) {
                        let mut state = self.state.blocking_lock();
                        state.reset();
                        state.load_nodes(nodes);
                    }
                    gui::menu::file::exit(ui);
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.spacing_mut().scroll.floating = false;
            let running = self.connection_state.modal(&self.modal, &self.tokio_handle, None, |ui| {
                ui.label("Load nodes");
                ui.label(self.file_path.as_ref().map(|a| a.to_string_lossy()).unwrap_or_default());
                if ui.button("Select File").clicked() {
                    if let Some(file_path) = gui::file_prompt("Load nodes", self.file_path.as_ref()).pick_file() {
                        self.file_path = Some(file_path);
                    }
                }
                ui.end_row();
            });

            if running {
                let mut state = self.state.blocking_lock();
                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    let per_row = 4;
                    egui::Grid::new("list")
                        .spacing([0.0, 64.0])
                        .show(ui, |ui| {
                            for node in state.nodes.iter_mut().filter(|a| a.sort.is_some()) {
                                let title = format!(
                                    "Node {:3}{}{}",
                                    node.address.as_node_address(),
                                    node.name.as_ref().map(|name| format!(" ({name})")).unwrap_or_default(),
                                    node.sort.as_ref().map(|sort| format!(" [{sort}]")).unwrap_or_default()
                                );
                                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                    if ui.heading(title).on_hover_text("Click to open in new window").clicked() {
                                        let index = usize::from(node.address.as_node_address());
                                        self.show_nodes[index] = true;
                                    }
                                    egui::Grid::new(node.address)
                                        .spacing([32.0, 0.0])
                                        .show(ui, |ui| {
                                            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                                gui::list_of_bytes(ui, per_row, "Inputs", gui::Mutable(&mut node.inputs), &node.labels.inputs);
                                            });
                                            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                                gui::list_of_bytes(ui, per_row, "Outputs", gui::ReadOnly(&node.outputs), &node.labels.outputs);
                                            });
                                        });
                                });
                                ui.end_row();
                            };
                        });
                });
                for (address, show) in self.show_nodes.iter_mut().enumerate() {
                    if *show {
                        Self::show_node(ctx, self.title.as_str(), &mut state.nodes[address], show);
                    }
                }
            } else if let Some(connection) = self.connection_state.try_get_connection() {
                self.title = format!("{}: {}", APP_TITLE, connection.name());
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.title.clone()));
                run_connection(connection, self.state.clone(), &self.tokio_handle);
                if let Some(file_path) = self.file_path.as_ref() {
                    match file::load_nodes(file_path) {
                        Err(error) => gui::modal_error(&self.modal, &error),
                        Ok(nodes) => self.state.blocking_lock().load_nodes(nodes)
                    }
                }
            }
        });
    }
}

impl App {
    fn show_node(ctx: &egui::Context, title: &str, node: &mut Node, show: &mut bool) {
        let title = node.name.as_ref().map_or_else(
            || format!("{title} - Node {}", node.address),
            |name| format!("{title} - Node {} ({})", node.address, name)
        );
        let viewport_id = egui::ViewportId::from_hash_of(format!("ShowNode{}", node.address));
        let viewport_builder = egui::ViewportBuilder::default().with_title(title).with_inner_size([600.0, 800.0]);

        ctx.show_viewport_immediate(viewport_id, viewport_builder, |ctx, _class| {
            egui::CentralPanel::default().show(ctx, |ui| {
                egui_extras::StripBuilder::new(ui)
                .sizes(egui_extras::Size::remainder(), 2)
                .cell_layout(egui::Layout::top_down_justified(egui::Align::Min))
                .horizontal(|mut strip| {
                    strip.cell(|ui| {
                        ui.heading("Inputs");
                        gui::list_of_bits(ui, gui::Mutable(&mut node.inputs), &node.labels.inputs);
                    });
                    strip.cell(|ui| {
                        ui.heading("Outputs");
                        gui::list_of_bits(ui, gui::ReadOnly(&node.outputs), &node.labels.outputs);
                    });
                });
        });

        if ctx.input(|i| i.viewport().close_requested()) {
                *show = false;
            }
        });
    }
}
