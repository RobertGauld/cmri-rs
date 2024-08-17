use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::trace;
use cmri_tools::gui;
use super::hub::{Hub, state::{State, ConnectionState}};

pub const APP_TITLE: &str = "CMRInet Hub";

#[expect(clippy::unwrap_used, clippy::missing_panics_doc)]
pub fn run(hub: Hub, state: Arc<Mutex<State>>, tokio_handle: tokio::runtime::Handle) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 400.0]),
        ..Default::default()
    };

    trace!("Running egui app.");
    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(move |cc| {
            let modal = {
                let style = egui_modal::ModalStyle { default_width: Some(500.0), ..Default::default() };
                egui_modal::Modal::new(&cc.egui_ctx, "Modal").with_style(&style)
            };
            Ok(Box::new(App {
                state,
                hub,
                new_connection: None,
                new_server: None,
                modal,
                tokio_handle
            }))
        })
    ).unwrap();
}

struct App {
    state: Arc<Mutex<State>>,
    hub: Hub,
    new_connection: Option<gui::connection::State>,
    new_server: Option<String>,
    modal: egui_modal::Modal,
    tokio_handle: tokio::runtime::Handle
}

impl eframe::App for App {
    #[expect(clippy::too_many_lines)]
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.modal.show_dialog();

        if self.new_server.is_some() {
            let mut cancel = false;
            let mut start = false;
            gui::modal_prompt(&self.modal, "Start Server",
                |ui| {
                    ui.label("Address");
                    ui.text_edit_singleline(self.new_server.as_mut().expect("Already checked it's Some"));
                },
                |modal, ui| {
                    if modal.suggested_button(ui, "Start").clicked() {
                        start = true;
                    }
                    if modal.caution_button(ui, "Cancel").clicked() {
                        cancel = true;
                    }
                }
            );
            if cancel {
                self.new_server = None;
            }
            if start {
                let address = self.new_server.take().expect("Already checked it's Some");
                let hub = self.hub.clone();
                if let Err(error) = self.tokio_handle.block_on( async move { hub.start_server(address.as_str()).await }) {
                    gui::modal_error(&self.modal, &error);
                }
            }
        }

        {
            let mut cancel = false;
            if let Some(connection_state) = self.new_connection.as_mut() {
                connection_state.modal(&self.modal, &self.tokio_handle, Some(&mut cancel), |_| {});
                if let Some(connection) = connection_state.try_get_connection() {
                    let hub = self.hub.clone();
                    self.tokio_handle.block_on( async move { hub.run_connection(connection); });
                }
            }
            if self.new_connection.is_some() && cancel {
                self.new_connection = None;
            }
        }

        let state = self.state.blocking_lock();
        egui::CentralPanel::default().show(ctx, |ui| {
            egui_extras::StripBuilder::new(ui)
                .size(egui_extras::Size::exact(150.0))
                .size(egui_extras::Size::remainder())
                .horizontal(|mut strip| {
                    strip.cell(|ui| {
                        if let Some(server) = state.server() {
                            ui.label(format!("Server running on:\n{server}"));
                        } else if ui.button("Start server").clicked() {
                            self.new_server = Some(String::from("127.0.0.1:7878"));
                        }

                        if ui.button("New connection").clicked() {
                            self.new_connection = Some(gui::connection::State::default());
                        }

                        ui.heading("Connections");
                        egui_extras::TableBuilder::new(ui)
                            .column(egui_extras::Column::exact(100.0))
                            .column(egui_extras::Column::exact(40.0))
                            .header(15.0, |mut header| {
                                header.col(|ui| { ui.label("Connection"); });
                                header.col(|ui| { ui.label("State"); });
                            })
                            .body(|mut body| {
                                for (name, state) in state.connections() {
                                    body.row(10.0, |mut row| {
                                        row.col(|ui| { ui.label(name); });
                                        row.col(|ui| {
                                            let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
                                            let radius = rect.size().x.min(rect.size().y) * 0.5;
                                            let center = rect.left_center() + egui::Vec2::new(radius, 0.0);
                                            match state {
                                                ConnectionState::Connected => {
                                                    ui.painter_at(rect).circle_filled(center, radius, egui::Color32::GREEN);
                                                    response.on_hover_text_at_pointer("OK");
                                                },
                                                ConnectionState::Disconnected => {
                                                    ui.painter_at(rect).circle_filled(center, radius, egui::Color32::GRAY);
                                                    response.on_hover_text_at_pointer("Disconnected");
                                                },
                                                ConnectionState::Errored(error) => {
                                                    ui.painter_at(rect).circle_filled(center, radius, egui::Color32::RED);
                                                    response.on_hover_text_at_pointer(error);
                                                }
                                            }
                                        });
                                    });
                                }
                            });
                    });

                    strip.strip(|strip| {
                        strip.size(egui_extras::Size::relative(0.5))
                            .size(egui_extras::Size::relative(0.5))
                            .vertical(|mut strip| {
                                strip.cell(|ui| {
                                    ui.heading(format!("Packets: {}", readable::num::Unsigned::from(state.frames().1)));
                                    Self::plot(ui, "Packets", &state.frames().2.as_vec());
                                });

                                strip.cell(|ui| {
                                    ui.heading(format!("Bytes: {}", readable_byte::readable_byte::b(state.bytes().1).to_string_as(true)));
                                    Self::plot(ui, "Bytes", &state.bytes().2.as_vec());
                                });
                            });
                    });
                });
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}

impl App {
    fn plot<T>(ui: &mut egui::Ui, title: &str, values: &[T]) where T: Into<f64> + Copy {
        let count = values.len();
        let values = values.iter()
            .enumerate()
            .map(|(x, y)| {
                #[expect(clippy::cast_precision_loss)]
                [-((count - x) as f64), Into::<f64>::into(*y)]
            })
            .collect::<Vec<[f64; 2]>>();
        egui_plot::Plot::new(title)
            .show_x(false)
            .show_y(false)
            .allow_zoom(false)
            .allow_drag(false)
            .allow_scroll(false)
            .x_axis_label("Time Ago")
            .x_grid_spacer(|_grid_input| {
                let mut vec = Vec::new();
                #[expect(clippy::cast_possible_truncation)]
                for i in (0..=(count as u16)).step_by(15).skip(1) {
                    if i % 60 == 0 {
                        vec.push(egui_plot::GridMark { value: -f64::from(i), step_size: 60.0 });
                    } else {
                        vec.push(egui_plot::GridMark { value: -f64::from(i), step_size: 15.0 });
                    }
                }
                vec
            })
            .x_axis_formatter(|grid_mark, _range| {
                #[expect(clippy::cast_possible_truncation)]
                let value = -grid_mark.value as f32;
                #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let minutes = (value / 60.0) as u8;
                #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let seconds = (value % 60.0) as u8;
                format!("{minutes}:{seconds:02}")
            })
            .y_axis_label(format!("{title} per second"))
            .y_axis_formatter(|grid_mark, _range| {
                #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                readable::num::Unsigned::from(grid_mark.value as u16).to_string()
            })
            .show(ui, |plot_ui| {
                plot_ui.line(egui_plot::Line::new(egui_plot::PlotPoints::from(values)).name(title));
            });
    }
}
