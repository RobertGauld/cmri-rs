use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::trace;
use cmri::Address;
use cmri_tools::{file, gui};
use crate::state::{State, run_connection};

pub const APP_TITLE: &str = "CMRInet Node";

#[expect(clippy::unwrap_used, clippy::missing_panics_doc, clippy::missing_errors_doc)]
pub fn run(cli_args: &clap::ArgMatches, tokio_handle: tokio::runtime::Handle) -> anyhow::Result<()> {
    let node_address = cli_args.get_one::<u8>("node-address").copied().unwrap_or_default().into();
    let nodes = cli_args.get_one::<std::path::PathBuf>("load-nodes")
        .map_or_else(
            || {
                let mut vec = Vec::with_capacity(128);
                for _ in 0..128 { vec.push(None) };
                Ok(vec)
            },
            |path| file::load_nodes(path)
        )?;

    let connection_state = tokio_handle.block_on(async { gui::connection::State::new(cli_args) });
    let file_path = cli_args.get_one::<std::path::PathBuf>("load-nodes").cloned();
    let state = Arc::new(Mutex::new(State::new()));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 800.0]),
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
                state,
                labels: file::Labels::default(),
                nodes,
                connection_state,
                node_address,
                file_path,
                modal,
                tokio_handle
            }))
        })
    ).unwrap();
    Ok(())
}

struct App {
    state: Arc<Mutex<State>>,
    labels: file::Labels,
    nodes: Vec<Option<file::Node>>,
    connection_state: gui::connection::State,
    node_address: usize,
    file_path: Option<std::path::PathBuf>,
    modal: egui_modal::Modal,
    tokio_handle: tokio::runtime::Handle
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.modal.show_dialog();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.spacing_mut().scroll.floating = false;
            let running = self.connection_state.modal(&self.modal, &self.tokio_handle, None, |ui| {
                ui.label("Node address");
                egui::ComboBox::from_id_source("node address")
                    .show_index(
                        ui,
                        &mut self.node_address,
                        128,
                        |i| self.nodes[i].as_ref().and_then(|node| node.name.as_ref()).map_or_else(
                            || format!("{i:3}"),
                            |name| format!("{i:3}: {name}")
                        )
                    );
                if let Some(new_nodes) = gui::menu::file::load_nodes(ui, &self.modal, &mut self.file_path) {
                    self.nodes = new_nodes;
                }
                ui.end_row();
            });

            if running {
                let mut state = self.state.blocking_lock();
                if state.initialised {
                    egui_extras::StripBuilder::new(ui)
                        .sizes(egui_extras::Size::remainder(), 2)
                        .cell_layout(egui::Layout::top_down_justified(egui::Align::Min))
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {
                                ui.heading("Inputs");
                                gui::list_of_bits(ui, gui::Mutable(&mut state.inputs), &self.labels.inputs);
                            });
                            strip.cell(|ui| {
                                ui.heading("Outputs");
                                gui::list_of_bits(ui, gui::ReadOnly(&state.outputs), &self.labels.outputs);
                            });
                        });
                } else {
                    self.modal.show(|ui| {
                        self.modal.title(ui, "Waiting for initilisation");
                        self.modal.body_and_icon(ui, "Waiting for initilisation packet to be sent by the controller.", egui_modal::Icon::Custom((String::from("⧗"), egui::Color32::from_rgb(150, 200, 210))));
                    });
                    self.modal.open();
                }
            } else if let Some(connection) = self.connection_state.try_get_connection() {
                let address = Address::try_from_node_address(u8::try_from(self.node_address).expect("Already checked it's in range")).expect("Already checked it's in range");
                let node = self.nodes[self.node_address].take();
                self.nodes = Vec::new(); // No need to keep all the nodes around
                let title = node.as_ref().and_then(|node| node.name.as_ref()).map_or_else(
                    || format!("{}: {} node {}", APP_TITLE, connection.name(), address),
                    |name| format!("{}: {} (node {}) on {}", APP_TITLE, name, address, connection.name())
                );
                if let Some(node) = node {
                    self.labels = node.labels;
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
                run_connection(self.state.clone(), connection, address, &self.tokio_handle);
            }
        });
    }
}
