use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{warn, trace};
use cmri::{NodeSort, node_configuration::node_cards::NodeCard};
use cmri_tools::{file, gui};
use crate::monitor::{State, Statistics, Node, run_connection, run_ticker};

pub const APP_TITLE: &str = "CMRInet Monitor";

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

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([750.0, 500.0]),
        ..eframe::NativeOptions::default()
    };

    trace!("Running egui app.");
    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(move |cc| {
            let modal = {
                let style = egui_modal::ModalStyle { default_width: Some(400.0), ..Default::default() };
                egui_modal::Modal::new(&cc.egui_ctx, "Modal").with_style(&style)
            };
            Ok(Box::new(App {
                state: Arc::new(Mutex::new(State::default())),
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
                    gui::menu::file::save_nodes(ui, &self.modal, &mut self.file_path, ||
                        self.state.blocking_lock().nodes().iter()
                            .filter_map(|n| file::Node::try_from(n).ok() )
                            .collect()
                    );
                    gui::menu::file::exit(ui);
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
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
                let state = self.state.blocking_lock();
                egui_extras::StripBuilder::new(ui)
                    .size(egui_extras::Size::exact(150.0))
                    .size(egui_extras::Size::remainder())
                    .vertical(|mut strip| {
                        strip.strip(|builder| {
                            builder
                                .size(egui_extras::Size::exact(300.0))
                                .size(egui_extras::Size::remainder())
                                .horizontal(|mut strip| {
                                    strip.cell(|ui| {
                                        Self::render_statistics(ui, state.statistics());
                                    });
                                    strip.cell(|ui| {
                                        let data = [
                                            ("Initialization", state.statistics().initialization_packets().2.as_vec()),
                                            ("Poll Request", state.statistics().poll_packets().2.as_vec()),
                                            ("Receive Data", state.statistics().receive_data_packets().2.as_vec()),
                                            ("Transmit Data", state.statistics().transmit_data_packets().2.as_vec()),
                                            #[cfg(feature = "experimenter")]
                                            ("Unknown", state.statistics().unknown_packets().2.as_vec())
                                        ];
                                        Self::render_plot(ui, data.as_slice());
                                    });
                                });
                        });

                        strip.cell(|ui| {
                            Self::render_list(ui, &state, &mut self.show_nodes);
                        });
                    });

                for (address, show) in self.show_nodes.iter_mut().enumerate() {
                    if *show {
                        Self::show_node(ctx, address, &state.nodes()[address], show);
                    }
                }
            } else if let Some(connection) = self.connection_state.try_get_connection() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!("{}: {}", APP_TITLE, connection.name())));
                run_connection(connection, self.state.clone(), &self.tokio_handle);
                run_ticker(self.state.clone(), &self.tokio_handle);
                if let Some(file_path) = self.file_path.as_ref() {
                    match file::load_nodes(file_path) {
                        Err(error) => gui::modal_error(&self.modal, &error),
                        Ok(nodes) => self.state.blocking_lock().load_nodes(nodes)
                    }
                }
            }
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

impl App {
    fn show_node(ctx: &egui::Context, address: usize, node: &Node, show: &mut bool) {
        let title = node.name().map_or_else(
            || format!("Node {address}"),
            |name| format!("Node {address} ({name})")
        );
        let viewport_id = egui::ViewportId::from_hash_of(format!("ShowNode{address}"));
        let viewport_builder = egui::ViewportBuilder::default().with_title(title).with_inner_size([1200.0, 600.0]);

        ctx.show_viewport_immediate(viewport_id, viewport_builder, |ctx, _class| {
            egui::CentralPanel::default().show(ctx, |ui| {
                egui_extras::StripBuilder::new(ui)
                    .size(egui_extras::Size::exact(150.0))
                    .size(egui_extras::Size::remainder())
                    .vertical(|mut strip| {
                        strip.strip(|builder| {
                            builder
                                .size(egui_extras::Size::exact(300.0))
                                .size(egui_extras::Size::exact(300.0))
                                .size(egui_extras::Size::remainder())
                                .horizontal(|mut strip| {
                                    strip.cell(|ui| {
                                        ui.push_id(format!("Node{address}Statistics"), |ui| {
                                            Self::render_statistics(ui, node.statistics());
                                        });
                                    });
                                    strip.cell(|ui| {
                                        Self::render_node_details(ui, node);
                                    });
                                    strip.cell(|ui| {
                                        let data = [
                                            ("Initialization", node.statistics().initialization_packets().2.as_vec()),
                                            ("Poll Request", node.statistics().poll_packets().2.as_vec()),
                                            ("Receive Data", node.statistics().receive_data_packets().2.as_vec()),
                                            ("Transmit Data", node.statistics().transmit_data_packets().2.as_vec()),
                                            #[cfg(feature = "experimenter")]
                                            ("Unknown", node.statistics().unknown_packets().2.as_vec())
                                        ];
                                        Self::render_plot(ui, data.as_slice());
                                    });
                                });
                        });
                        strip.strip(|builder| {
                        builder
                            .size(egui_extras::Size::relative(0.5))
                            .size(egui_extras::Size::relative(0.5))
                            .horizontal(|mut strip| {
                                let per_row = 4;
                                strip.cell(|ui| {
                                    if let Some(data) = node.inputs() {
                                        gui::list_of_bytes(ui, per_row, "Inputs", gui::ReadOnly(data), &node.labels().inputs);
                                    }
                                });
                                strip.cell(|ui| {
                                    if let Some(data) = node.outputs() {
                                        gui::list_of_bytes(ui, per_row, "Outputs", gui::ReadOnly(data), &node.labels().outputs);
                                    }
                                });
                            });
                        });
                    });
            });

            if ctx.input(|i| i.viewport().close_requested()) {
                *show = false;
            }
        });
    }

    #[inline]
    fn render_statistics(ui: &mut egui::Ui, statistics: &Statistics) {
        let total_packets = statistics.packets().1;
        let do_percent = total_packets > 0;

        let layout = egui::Layout { main_align: egui::Align::Center, cross_align: egui::Align::Center, main_justify: true, cross_justify: true, ..*ui.layout() };
        ui.allocate_ui_with_layout(ui.available_size(), layout, |ui| {
        egui::Frame::none()
            .show(ui, |ui| {
                egui_extras::TableBuilder::new(ui)
                    .striped(true)
                    .cell_layout(egui::Layout::default().with_cross_align(egui::Align::RIGHT))
                    .column(egui_extras::Column::auto().at_least(125.0))
                    .column(egui_extras::Column::auto().at_least(75.0))
                    .column(egui_extras::Column::auto().at_least(50.0))
                    .body(|mut body| {
                        body.row(15.00, |mut row| {
                            row.col(|ui| { ui.label("Packets"); });
                            row.col(|ui| { ui.label(readable::num::Unsigned::from(total_packets).as_str()); });
                        });
                        body.row(15.00, |mut row| {
                            row.col(|ui| { ui.label("Bad Packets"); });
                            row.col(|ui| { ui.label(readable::num::Unsigned::from(statistics.bad_packets().1).as_str()); });
                            if do_percent {
                                row.col(|ui| { ui.label(format!("{}%", (statistics.bad_packets().1  * 100) / total_packets)); });
                            }
                        });
                        body.row(15.00, |mut row| {
                            row.col(|ui| { ui.label("Initialization Packets"); });
                            row.col(|ui| { ui.label(readable::num::Unsigned::from(statistics.initialization_packets().1).as_str()); });
                            if do_percent {
                                row.col(|ui| { ui.label(format!("{}%", (statistics.initialization_packets().1  * 100) / total_packets)); });
                            }
                        });
                        body.row(15.00, |mut row| {
                            row.col(|ui| { ui.label("Poll Packets"); });
                            row.col(|ui| { ui.label(readable::num::Unsigned::from(statistics.poll_packets().1).as_str()); });
                            if do_percent {
                                row.col(|ui| { ui.label(format!("{}%", (statistics.poll_packets().1 * 100) / total_packets)); });
                            }
                        });
                        body.row(15.00, |mut row| {
                            row.col(|ui| { ui.label("Receive Data Packets"); });
                            row.col(|ui| { ui.label(readable::num::Unsigned::from(statistics.receive_data_packets().1).as_str()); });
                            if do_percent {
                                row.col(|ui| { ui.label(format!("{}%", (statistics.receive_data_packets().1  * 100) / total_packets)); });
                            }
                        });
                        body.row(15.00, |mut row| {
                            row.col(|ui| { ui.label("Transmit Data Packets"); });
                            row.col(|ui| { ui.label(readable::num::Unsigned::from(statistics.transmit_data_packets().1).as_str()); });
                            if do_percent {
                                row.col(|ui| { ui.label(format!("{}%", (statistics.transmit_data_packets().1  * 100) / total_packets)); });
                            }
                        });
                        #[cfg(feature = "experimenter")]
                        body.row(15.00, |mut row| {
                            row.col(|ui| { ui.label("Unknown Packets"); });
                            row.col(|ui| { ui.label(readable::num::Unsigned::from(statistics.unknown_packets().1).as_str()); });
                            if do_percent {
                                row.col(|ui| { ui.label(format!("{}%", (statistics.unknown_packets().1  * 100) / total_packets)); });
                            }
                        });
                    });
            });
        });
    }

    #[expect(clippy::missing_panics_doc, clippy::cast_possible_truncation, clippy::cast_lossless, clippy::cast_sign_loss)]
    fn render_plot(ui: &mut egui::Ui, data: &[(&str, Vec<u16>)]) {
        let data = data.iter().map(|(title, values)| {
            let count = values.len() as u16;
            let values = values.iter()
                .enumerate()
                .map(|(x, y)| [-f64::from(count - u16::try_from(x).expect("x comes from READINGS_SIZE which is < u16::MAX")), f64::from(*y)])
                .collect::<Vec<[f64; 2]>>();
            (title, values)
        }).collect::<Vec<_>>();

        egui_plot::Plot::new("Packets")
            .show_x(false)
            .show_y(false)
            .allow_zoom(false)
            .allow_drag(false)
            .allow_scroll(false)
            .x_axis_label("Time Ago")
            .x_grid_spacer(|grid_input| {
                let mut vec = Vec::new();
                for i in (0..((grid_input.bounds.1 - grid_input.bounds.0).abs().ceil() as u16)).step_by(15).skip(1) {
                    if i % 60 == 0 {
                        vec.push(egui_plot::GridMark { value: -(i as f64), step_size: 60.0 });
                    } else {
                        vec.push(egui_plot::GridMark { value: -(i as f64), step_size: 15.0 });
                    }
                }
                vec
            })
            .x_axis_formatter(|grid_mark, _range| {
                let minutes = (-grid_mark.value / 60.0) as u8;
                let seconds = (-grid_mark.value % 60.0) as u8;
                format!("{minutes}:{seconds:02}")
            })
            .y_axis_label("Packets per second")
            .y_axis_formatter(|grid_mark, _range| {
                readable::num::Unsigned::from(grid_mark.value as u16).to_string()
            })
            .legend(egui_plot::Legend::default().position(egui_plot::Corner::LeftTop))
            .show(ui, |plot_ui| {
                for (title, values) in data {
                    plot_ui.line(egui_plot::Line::new(egui_plot::PlotPoints::from(values)).name(title));
                }
            });
    }

    fn render_list(ui: &mut egui::Ui, state: &State, show_nodes: &mut [bool; 128]) {
        ui.spacing_mut().scroll.floating = false;
        let total_packets = state.statistics().packets().1;
        egui_extras::TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .max_scroll_height(200.0)
            .column(egui_extras::Column::auto().at_least(50.0))
            .column(egui_extras::Column::auto().at_least(100.0))
            .column(egui_extras::Column::auto().at_least(250.0))
            .column(egui_extras::Column::auto().at_least(80.0))
            .column(egui_extras::Column::auto().at_least(80.0))
            .column(egui_extras::Column::auto().at_least(80.0))
            .column(egui_extras::Column::auto().at_least(50.0))
            .header(40.0, |mut header| {
                header.col(|ui| { ui.label("Address"); });
                header.col(|ui| { ui.label("Friendly Name"); });
                header.col(|ui| { ui.label("Sort"); });
                header.col(|ui| { ui.label("Transmission Delay (µs)"); });
                header.col(|ui| { ui.label("Packets"); });
                header.col(|ui| { ui.label("Initialization Count"); });
            })
            .body(|mut body| {
                for (address, node) in state.nodes().iter().enumerate() {
                    if node.has_been_seen() || node.sort().is_some() {
                        let sort = node.sort().map_or(String::new(), |i| format!("{i}"));
                        body.row(20.0, |mut row| {
                            row.col(|ui| { ui.label(format!("{address:3}")); });
                            row.col(|ui| {
                                if let Some(name) = node.name() {
                                    ui.label(name);
                                }
                            });
                            row.col(|ui| {
                                if let Some(sort) = node.sort() {
                                    let inputs = sort.configuration().input_bits();
                                    let outputs = sort.configuration().output_bits();
                                    ui.label(format!("{sort} with {inputs} inputs and {outputs} outputs"));
                                }
                            });
                            row.col(|ui| {
                                let value = node.sort()
                                    .map(|i| readable::num::Unsigned::from(u32::from(i.configuration().transmit_delay()) * 10).to_string())
                                    .unwrap_or_default();
                                ui.label(value);
                            });
                            row.col(|ui| {
                                if total_packets > 0 {
                                    ui.label(format!("{} ({}%)", readable::num::Unsigned::from(node.statistics().packets().1).as_str(), (node.statistics().packets().1 * 100) / total_packets));
                                } else {
                                    ui.label("0");
                                }
                            });
                            row.col(|ui| {
                                ui.label(node.initialization_count().to_string());
                            });
                            row.col(|ui| {
                                if ui.button("Open").on_hover_text_at_pointer(format!("Open node {address} ({})", &sort)).clicked() {
                                    show_nodes[address] = true;
                                }
                            });
                        });
                    }
                }
            });
    }

    fn render_node_details(ui: &mut egui::Ui, node: &Node) {
        if let Some(sort) = node.sort() {
            let configuration = sort.configuration();
            ui.heading(sort.to_string());
            ui.label(format!(
                "{} inputs, {} outputs, {}µs transmission delay",
                configuration.input_bits(),
                configuration.output_bits(),
                readable::num::Unsigned::from(u32::from(configuration.transmit_delay()) * 10)
            ));
            match sort {
                NodeSort::Usic { configuration } => {
                    ui.label(format!(
                        "24 bit cards\n{}",
                        configuration.cards().iter().filter_map(|c| match c {
                            NodeCard::Input => Some("I"),
                            NodeCard::Output => Some("O"),
                            NodeCard::None => None
                        }).collect::<Vec<_>>().join(" ")
                    ));
                },
                NodeSort::Susic { configuration } => {
                    let mut string = String::with_capacity(128);
                    string.push_str("24 bit cards\n");
                    configuration.cards().chunks(4).for_each(|chunk| {
                        for card in chunk {
                            match card {
                                NodeCard::Input => string.push('I'),
                                NodeCard::Output => string.push('O'),
                                NodeCard::None => string.push('.')
                            }
                        }
                        string.push(' ');
                    });
                    ui.label(string.trim());
                },
                NodeSort::Smini { configuration } => {
                    let oscillating_pairs = configuration.oscillating_pairs();
                    ui.label("Oscillating pairs");
                    ui.label(format!("\tCard 0: A:{:08b} B:{:08b} C:{:08b}", oscillating_pairs[0], oscillating_pairs[1], oscillating_pairs[2]));
                    ui.label(format!("\tCard 1: A:{:08b} B:{:08b} C:{:08b}", oscillating_pairs[3], oscillating_pairs[4], oscillating_pairs[5]));
                }
                NodeSort::Cpnode { configuration } => {
                    let options = configuration.options().bits();
                    ui.label(format!("Options: {options} {options:#06x} 0b{}", options.to_be_bytes().map(|a| format!("{a:08b}")).join(" ")));
                    ui.label(configuration.options().iter_names().map(|(a, _o)| format!("\t• {a}")).collect::<Vec<String>>().join("\n"));
                },
                NodeSort::Cpmega { configuration } => {
                    let options = configuration.options().bits();
                    ui.label(format!("Options: {options} {options:#06x} 0b{}", options.to_be_bytes().map(|a| format!("{a:08b}")).join(" ")));
                    ui.label(configuration.options().iter_names().map(|(a, _o)| format!("\t• {a}")).collect::<Vec<String>>().join("\n"));
                },
                #[cfg(feature = "experimenter")]
                NodeSort::Unknown { .. } => ()
            }
        }
    }
}
