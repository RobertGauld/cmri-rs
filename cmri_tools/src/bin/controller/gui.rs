use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{trace, error, warn};
use cmri::node_configuration::node_cards::NodeCard;
use cmri_tools::{file, gui};
use crate::controller::{State, Node, run_connection};

pub const APP_TITLE: &str = "CMRInet Controller";

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
                add_node: AddNode::default(),
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
    add_node: AddNode,
    modal: egui_modal::Modal,
    tokio_handle: tokio::runtime::Handle
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let add_node_modal = self.add_node.modal(ctx, &self.state, &self.modal);
        self.modal.show_dialog();

        egui::TopBottomPanel::top("MenuPanel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if let Some(nodes) = gui::menu::file::load_nodes(ui, &self.modal, &mut self.file_path) {
                        let mut state = self.state.blocking_lock();
                        state.reset();
                        state.load_nodes(nodes);
                    }
                    gui::menu::file::save_nodes(ui, &self.modal, &mut self.file_path, ||
                        self.state.blocking_lock().nodes.iter()
                            .filter_map(|n| n.as_ref().map(file::Node::from) )
                            .collect()
                    );
                    gui::menu::file::exit(ui);
                });
                if ui.button("Add Node").clicked() {
                    add_node_modal.open();
                }
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
                            for node in state.nodes.iter_mut().filter_map(|a| a.as_mut()) {
                                let title = format!(
                                    "Node {:3}{} [{}]",
                                    node.address.as_node_address(),
                                    node.name.as_ref().map(|name| format!(" ({name})")).unwrap_or_default(),
                                    node.sort
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
                                                gui::list_of_bytes(ui, per_row, "Inputs", gui::ReadOnly(&node.inputs), &node.labels.inputs);
                                            });
                                            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                                gui::list_of_bytes(ui, per_row, "Outputs", gui::Mutable(&mut node.outputs), &node.labels.outputs);
                                            });
                                        });
                                });
                                ui.end_row();
                            };
                        });
                });
                for (index, show) in self.show_nodes.iter_mut().enumerate().filter(|(_, b)| **b) {
                    if let Some(node) = state.nodes[index].as_mut() {
                        Self::show_node(ctx, self.title.as_str(), node, show);
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
                        gui::list_of_bits(ui, gui::ReadOnly(&node.inputs), &node.labels.inputs);
                    });
                    strip.cell(|ui| {
                        ui.heading("Outputs");
                        gui::list_of_bits(ui, gui::Mutable(&mut node.outputs), &node.labels.outputs);
                    });
                });
        });

        if ctx.input(|i| i.viewport().close_requested()) {
                *show = false;
            }
        });
    }
}

struct AddNode {
    address_index: usize,
    name: String,
    config_index: u8,
    #[expect(clippy::type_complexity)]
    configs: (
        [NodeCard; 64], // USIC cards
        [NodeCard; 64], // SUSI cards
        [bool; 48],     // SMINI oscillating pairs
        (u8, u8),       // CPNODE (inputs, outputs)
        (u8, u8)        // CPMEGA (inputs, outputs)
    )
}
impl AddNode {
    #[expect(clippy::missing_panics_doc)]
    fn modal(&mut self, ctx: &egui::Context, state: &Arc<Mutex<State>>, error_modal: &egui_modal::Modal) -> egui_modal::Modal {
        let available_addresses = state.blocking_lock().available_node_addresses();
        let modal = egui_modal::Modal::new(ctx, "AddNode")
            .with_style(&egui_modal::ModalStyle { default_width: Some(500.0), default_height: Some(400.0), ..Default::default() });
        modal.show(|ui| {
            modal.title(ui, "Add Node");
            modal.frame(ui, |ui| {
                egui_extras::StripBuilder::new(ui)
                    .size(egui_extras::Size::initial(50.0))
                    .size(egui_extras::Size::initial(20.0))
                    .size(egui_extras::Size::initial(130.0))
                    .vertical(|mut strip| {
                        strip.cell(|ui| {
                            egui_extras::TableBuilder::new(ui)
                                .striped(false)
                                .resizable(false)
                                .columns(egui_extras::Column::remainder(), 2)
                                .body(|mut body| {
                                    body.row(15.0, |mut row| {
                                        row.col(|ui| { ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label("Address"); }); });
                                        row.col(|ui| {
                                            egui::ComboBox::from_id_source("address")
                                                .show_index(ui, &mut self.address_index, available_addresses.len(), |i| format!("{:3}", available_addresses[i]));
                                        });
                                    });
                                    body.row(15.0, |mut row| {
                                        row.col(|ui| { ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label("Name"); }); });
                                        row.col(|ui| { ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| { ui.text_edit_singleline(&mut self.name); }); });
                                    });
                            });
                        });

                        strip.strip(|builder| {
                            builder
                                .sizes(egui_extras::Size::remainder(), 5)
                                .horizontal(|mut strip| {
                                    strip.cell(|ui| {
                                        ui.radio_value(&mut self.config_index, 0, "(S)USIC").on_hover_text("A USIC or SUSIC with 24 bit cards.");
                                    });
                                    strip.cell(|ui| {
                                        ui.radio_value(&mut self.config_index, 1, "SUSIC").on_hover_text("A SUSIC with 32 bit cards.");
                                    });
                                    strip.cell(|ui| {
                                        ui.radio_value(&mut self.config_index, 2, "SMINI").on_hover_text("An SMINI with 24 inputs and 48 outputs.");
                                    });
                                    strip.cell(|ui| {
                                        ui.radio_value(&mut self.config_index, 3, "CPNODE").on_hover_text("A CPNODE with 16 to 144 input/outputs.");
                                    });
                                    strip.cell(|ui| {
                                        ui.radio_value(&mut self.config_index, 4, "CPMEGA").on_hover_text("A CPMEGA with 0 to 192 input/outputs.");
                                    });
                                });
                        });

                        strip.cell(|ui| {
                            match self.config_index {
                                0 => Self::sic_config(ui, &mut self.configs.0, 24),
                                1 => Self::sic_config(ui, &mut self.configs.1, 32),
                                2 => Self::smini_config(ui, &mut self.configs.2),
                                3 => Self::cp_config(ui, &mut self.configs.3, 16..=144),
                                4 => Self::cp_config(ui, &mut self.configs.4, 0..=192),
                                _ => unreachable!()
                            }
                        });
                    });
            });
            modal.buttons(ui, |ui| {
                if modal.suggested_button(ui, "Ok").clicked {
                    let node_sort: Result<cmri::NodeSort, cmri::node_configuration::InvalidConfigurationError> = match self.config_index {
                        0 => cmri::NodeSort::try_new_usic(0, &self.configs.0).map_err(Into::into),
                        1 => cmri::NodeSort::try_new_usic(0, &self.configs.1).map_err(Into::into),
                        2 => cmri::NodeSort::try_new_smini(0, Self::build_oscillating_pairs(&self.configs.2)).map_err(Into::into),
                        3 => cmri::NodeSort::try_new_cpnode(0, cmri::node_configuration::CpnodeOptions::default(), self.configs.3.0, self.configs.3.1).map_err(Into::into),
                        4 => cmri::NodeSort::try_new_cpmega(0, cmri::node_configuration::CpmegaOptions::default(), self.configs.4.0, self.configs.4.1).map_err(Into::into),
                        _ => unreachable!()
                    };
                    match node_sort {
                        Err(error) => {
                            error!("{error}");
                            error_modal.dialog()
                                .with_title("Couldn't add node")
                                .with_body(error.to_string())
                                .with_icon(egui_modal::Icon::Error)
                                .open();
                        },
                        Ok(node_sort) => {
                            let address = available_addresses[self.address_index];
                            let node = Node::new(
                                cmri::Address::try_from_node_address(address).expect("Only valid options are selectable."),
                                node_sort,
                                if self.name.is_empty() { None } else { Some(self.name.clone()) }
                            );
                            state.blocking_lock().nodes[address as usize] = Some(node);
                            self.address_index = 0;
                        }
                    }
                }
                if modal.caution_button(ui, "Cancel").clicked() {
                    *self = Self::default();
                }
            });
        });
        modal
    }

    fn sic_config(ui: &mut egui::Ui, config: &mut[NodeCard; 64], bits: u8) {
        ui.label("Click the x (none), I (input), or O (output) tochange the type of card in each position.");
        egui_extras::TableBuilder::new(ui)
            .striped(false)
            .resizable(false)
            .columns(egui_extras::Column::remainder(), 64)
            .header(15.0, |mut row| {
                for i in 0..64 {
                    row.col(|ui| {
                        ui.label(format!("{i:02}"));
                    });
                }
            }).body(|mut body| {
                body.row(15.0, |mut row| {
                    for card in config.iter_mut() {
                        let (label, next) = match card {
                            NodeCard::None   => ("x", NodeCard::Input),
                            NodeCard::Input  => ("I", NodeCard::Output),
                            NodeCard::Output => ("O", NodeCard::None)
                        };
                        row.col(|ui| {
                            if ui.label(label).clicked() {
                                *card = next;
                            }
                        });
                    }
                });
            });

        match cmri::node_configuration::node_cards::NodeCards::try_new(config) {
            Ok(cards) => {
                ui.label(format!(
                    "{} input cards, {} output cards, {} slots available. ({} inputs and {} outputs)",
                    cards.input_cards(),
                    cards.output_cards(),
                    64 - cards.len(),
                    cards.input_cards() * bits,
                    cards.output_cards() * bits
                ));
            },
            Err(error) => {
                ui.label(egui::RichText::new(error.to_string()).color(egui::Color32::RED).strong());
            }
        }
    }

    fn smini_config(ui: &mut egui::Ui, config: &mut[bool; 48]) {
        egui_extras::TableBuilder::new(ui)
            .striped(false)
            .resizable(false)
            .auto_shrink(egui::Vec2b { x: true, y: false })
            .column(egui_extras::Column::remainder())
            .columns(egui_extras::Column::remainder(), 8)
            .body(|mut body| {
                body.row(15.0, |mut row| {
                    row.col(|ui| { ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label("Oscillating pairs:"); }); });
                    row.col(|ui| { ui.label("0"); });
                    row.col(|ui| { ui.label("1"); });
                    row.col(|ui| { ui.label("2"); });
                    row.col(|ui| { ui.label("3"); });
                    row.col(|ui| { ui.label("4"); });
                    row.col(|ui| { ui.label("5"); });
                    row.col(|ui| { ui.label("6"); });
                    row.col(|ui| { ui.label("7"); });
                });

                body.row(15.0, |mut row| {
                    row.col(|ui| { ui.label("Card 0 Port A"); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[0])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[1])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[2])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[3])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[4])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[5])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[6])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[7])); });
                });

                body.row(15.0, |mut row| {
                    row.col(|ui| { ui.label("Card 0 Port B"); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[8])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[9])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[10])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[11])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[12])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[13])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[14])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[15])); });
                });

                body.row(15.0, |mut row| {
                    row.col(|ui| { ui.label("Card 0 Port C"); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[16])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[17])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[18])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[19])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[20])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[21])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[22])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[23])); });
                });

                body.row(15.0, |mut row| {
                    row.col(|ui| { ui.label("Card 1 Port A"); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[24])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[25])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[26])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[27])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[28])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[29])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[30])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[31])); });
                });

                body.row(15.0, |mut row| {
                    row.col(|ui| { ui.label("Card 1 Port B"); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[32])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[33])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[34])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[35])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[36])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[37])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[38])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[39])); });
                });

                body.row(15.0, |mut row| {
                    row.col(|ui| { ui.label("Card 1 Port C"); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[40])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[41])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[42])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[43])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[44])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[45])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[46])); });
                    row.col(|ui| { ui.add(egui::Checkbox::without_text(&mut config[47])); });
                });
            });

        let pairs = Self::build_oscillating_pairs(config);
        if let Err(error) = cmri::node_configuration::SminiConfiguration::get_oscillating_pairs_count(&pairs) {
            ui.label(egui::RichText::new(error.to_string()).color(egui::Color32::RED).strong());
        }
    }

    fn cp_config(ui: &mut egui::Ui, config: &mut(u8, u8), allowed_total: core::ops::RangeInclusive<u8>) {
        let allowed_bytes = allowed_total.end() / 8;
        egui_extras::TableBuilder::new(ui)
            .striped(false)
            .resizable(false)
            .columns(egui_extras::Column::remainder(), 2)
            .body(|mut body| {
                body.row(15.0, |mut row| {
                    row.col(|ui| { ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label("Input bytes:"); }); });
                    row.col(|ui| { ui.add(egui::Slider::new(&mut config.0, 0..=(allowed_bytes.saturating_sub(config.1)))); });
                });
                body.row(15.0, |mut row| {
                    row.col(|ui| { ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label("Output bytes:"); }); });
                    row.col(|ui| { ui.add(egui::Slider::new(&mut config.1, 0..=(allowed_bytes.saturating_sub(config.0)))); });
                });
                body.row(15.0, |mut row| {
                    row.col(|ui| { ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label("Available bytes:"); }); });
                    row.col(|ui| { ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| { ui.label(format!("{} of {}", allowed_bytes.saturating_sub(config.0).saturating_sub(config.1), allowed_bytes)); }); });
                });
            });

        let total_bits = (u16::from(config.0) + u16::from(config.1)).saturating_mul(8);
        if !allowed_total.contains(&(total_bits.clamp(0, 255) as u8)) {
            let message = if *allowed_total.start() > 0 {
                format!("Total of inputs and outputs ({total_bits}) must be between {} and {} (inclusive).", allowed_total.start(), allowed_total.end())
            } else {
                format!("Total of inputs and outputs ({total_bits}) must be {} or less.", allowed_total.end())
            };
            ui.label(egui::RichText::new(message).color(egui::Color32::RED).strong());
        }
    }

    fn build_oscillating_pairs(bits: &[bool]) -> [u8; 6] {
        let mut oscillating_pairs = [0; 6];
        for (i, byte) in oscillating_pairs.iter_mut().enumerate() {
            for j in 0..8 {
                *byte >>= 1;
                if bits[(i * 8) + j] { *byte += 0b1000_0000; }
            }
        }
        oscillating_pairs
    }
}

impl Default for AddNode {
    fn default() -> Self {
        Self {
            address_index: 0,
            name: String::new(),
            config_index: 0,
            configs: (
                [NodeCard::None; 64],
                [NodeCard::None; 64],
                [false; 48],
                (0, 0),
                (0, 0)
            )
        }
    }
}
