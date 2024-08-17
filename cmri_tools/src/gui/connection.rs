#![allow(clippy::doc_markdown)]
//! Prompt the user for connection settings using a modal.
//!
//! # Example
//!
//! // Get from parsed cli options.
//! let connection_state = tokio_handle.block_on(async { State::new(cli_args) });
//!
//! // Prompt the use if needed.
//! let connected = connection_state.modal(..);
//!
//! // We may already have a connection.
//! if let Some(connection) = connection_state.try_get_connection() {
//!     todo!("Use connection");
//! }
//!
//! if connected {
//!     todo!("Render UI");
//! }

use tracing::{error, trace};
use crate::connection::Connection;
use crate::gui;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Sort {
    Serial,
    Network
}

#[expect(clippy::large_enum_variant)]
enum OptionsResult {
    None(Options),
    Ok(Connection),
    Err((Options, std::io::Error))
}

/// Options for creating a new `Connection`.
#[derive(Debug, Clone)]
pub struct Options {
    sort: Sort,
    network: String,
    serial: (String, u32),
    error: Option<String>,
    serial_ports: Vec<String>
}

impl Options {
    /// Create a new `Options` from a `clap::ArgMatches`.
    ///
    /// Assumes `cli_args` contains a single serial, a single network, or neither.
    /// If anything is passd on the cli attempts to make the connection, which may
    /// result in a None variant with the error field set.
    ///
    /// # Return
    ///
    /// * None(`Options`) with default values.
    /// * Err(`Options`, `std::io::Error`) with default values updated by the CLI arguments, and the error that resulted on trying to connect with them.
    /// * Ok(`Connection`) with a connection.
    ///
    /// # Panics
    ///
    /// * If a contained serial option can't be converted into a port and optional baud.
    #[must_use]
    fn from_cli(cli_args: &clap::ArgMatches) -> OptionsResult {
        trace!("Making a ConnectionOptions from CLI arguments: {cli_args:?}");
        let mut options = Self::default();
        let mut cli_args_present = false;

        if let Some(address) = cli_args.get_one::<String>("network") {
            options.set_network(address);
            cli_args_present = true;
        }
        if let Some(details) = cli_args.get_one::<String>("serial") {
            if let Ok((port, baud)) = crate::connection::port_baud_from_str(details) {
                options.set_serial(port, baud);
                cli_args_present = true;
            }
        }

        if cli_args_present {
            match options.try_connect() {
                Ok(connection) => OptionsResult::Ok(connection),
                Err(|error) => OptionsResult::Err((options, error))
            }
        } else {
            OptionsResult::None(options)
        }
    }

    /// Set the network details.
    ///
    /// For example if you're prompting because a previous connection attempt failed.
    fn set_network(&mut self, address: &str) {
        self.sort = Sort::Network;
        self.network = address.to_string();
    }

    /// Set the serial port details.
    ///
    /// For example if you're prompting because a previous connection attempt failed.
    fn set_serial(&mut self, port: impl Into<String>, baud: u32) {
        self.sort = Sort::Serial;
        self.serial = (port.into(), baud);
    }

    /// Try creating a new `Connection` from the settings within this `Options`.
    ///
    /// # Errors
    ///
    /// If the connction can't be made.
    fn try_connect(&mut self) -> std::io::Result<Connection> {
        match self.sort {
            Sort::Network => Connection::new_tcp_client(&self.network)
                                        .inspect_err(|error| {
                                            error!("Error connecting to {}: {error}", self.network);
                                            self.error = Some(error.to_string());
                                        }),
            Sort::Serial => Connection::new_serial_port(&self.serial.0, self.serial.1)
                                        .inspect_err(|error| {
                                            error!("Error connecting to {}: {error}", self.serial.0);
                                            self.error = Some(error.to_string());
                                        }),
        }
    }
}

impl Default for Options {
    fn default() -> Self {
        let mut serial_ports: Vec<String> = tokio_serial::available_ports()
            .unwrap_or_else(|_| Vec::new())
            .iter()
            .map(|spi| spi.port_name.clone())
            .collect();
        serial_ports.sort();

        Self {
            sort: Sort::Network,
            serial: (String::new(), cmri::DEFAULT_BAUD),
            network: String::from("127.0.0.1:7878"),
            error: None,
            serial_ports
        }
    }
}

/// The current state of an attempted connection.
#[derive(Debug)]
#[expect(clippy::large_enum_variant)]
pub enum State {
    /// The user is being prompted for connection settings.
    /// (connection options, whether to try connecting)
    Prompting((Options, bool)),
    /// A connection is being attempted.
    /// When `modal` is called, if the thread is finished the result dictates the following state.
    Connecting(Option<std::thread::JoinHandle<Result<Connection, (Options, std::io::Error)>>>),
    /// A connection has sucessfully been made.
    Connected(Connection),
    /// The connection has been made, and removed so is presumably being run by the application.
    Running
}

impl State {
    /// Create a new `ConnctionOptions` from a `clap::ArgMatches`.
    ///
    /// Assumes `cli_args` contains a single serial, a single network, or neither.
    /// If anything is passd on the cli attempts to make the connection, which may
    /// result in a None variant with the error field set.
    ///
    /// # Return
    ///
    /// * Prompting(`ConnectionOptions`, `bool`) with the options, and whether to try connecting.
    /// * Connected(`Connection`) with a connection.
    ///
    /// # Panics
    ///
    /// * If a contained serial option can't be converted into a port and optional baud.
    #[must_use]
    pub fn new(cli_args: &clap::ArgMatches) -> Self {
        match Options::from_cli(cli_args) {
            OptionsResult::None(options) => Self::Prompting((options, false)),
            OptionsResult::Err((options, _error)) => Self::Prompting((options, false)),
            OptionsResult::Ok(connection) => Self::Connected(connection)
        }
    }

    /// Populate the modal to prompt the user for connection (and other settings) settings, and change state is relevant.
    ///
    /// Returns true if the state is running, allowing this method to be used as a guard to populating the
    /// rest of the application.
    ///
    /// # Panics
    ///
    /// If a thread to create the connection can't be started.
    pub fn modal(&mut self, modal: &egui_modal::Modal, tokio_handle: &tokio::runtime::Handle, cancel: Option<&mut bool>, extra_content: impl FnOnce(&mut egui::Ui)) -> bool {
        match self {
            Self::Prompting((options, connect)) => {
                if *connect {
                    *self = Self::Connecting(Some(Self::connect(options.clone(), modal, tokio_handle.clone())));
                } else {
                    Self::prompt(options, modal, connect, cancel, extra_content);
                }
            },
            Self::Connecting(handle) => {
                modal.show(|ui| {
                    modal.title(ui, "Connecting");
                    modal.body_and_icon(ui, "Connecting.", egui_modal::Icon::Info);
                });
                modal.open();
                if handle.as_ref().is_some_and(std::thread::JoinHandle::is_finished) {
                    match handle.take().expect("Already checked it's a some").join() {
                        Err(error) => {
                            panic!("Couldn't start connect thread: {error:?}");
                        },
                        Ok(Err((options, error))) => {
                            error!("Couldn't make connection: {error}");
                            *self = Self::Prompting((options, false));
                        }
                        Ok(Ok(connection)) => {
                            *self = Self::Connected(connection);
                        }
                    }
                }
            },
            Self::Connected(_) => {
                modal.show(|ui| {
                    modal.title(ui, "Connected");
                    modal.body_and_icon(ui, "Connected.", egui_modal::Icon::Info);
                });
                modal.open();
            },
            Self::Running => return true
        }
        false
    }

    /// Try getting the created connection.
    ///
    /// # Returns
    ///
    /// * None if the current state isn't `Connected`.
    /// * Some if the current state is `Connected`,
    ///   also removes the `Connection` and sets the state to `Running`.
    pub fn try_get_connection(&mut self) -> Option<Connection> {
        match self {
            Self::Connected(_) => {
                if let Self::Connected(connection) = std::mem::replace(self, Self::Running) {
                    Some(connection)
                } else {
                    unreachable!()
                }
            },
            _ => None
        }
    }

    fn prompt(options: &mut Options, modal: &egui_modal::Modal, connect: &mut bool, cancel: Option<&mut bool>, extra_content: impl FnOnce(&mut egui::Ui)) {
        gui::modal_prompt(
            modal,
            "Connect",
            |ui| {
                if let Some(error) = options.error.as_ref() {
                    ui.label(egui::RichText::new(format!("ERROR: {error}")).color(egui::Color32::RED));
                }
                egui::Grid::new("grid").show(ui, |ui| {
                    ui.radio_value(&mut options.sort, Sort::Serial, "Serial");
                    let serial_port = egui::ComboBox::from_id_source("serial_port")
                        .selected_text(&options.serial.0)
                        .show_ui(ui, |ui| {
                            for port in &options.serial_ports {
                                ui.selectable_value(&mut options.serial.0, port.clone(), port);
                            }
                        });
                    let serial_baud = egui::ComboBox::from_id_source("serial_baud")
                        .selected_text(readable::num::Unsigned::from(options.serial.1).as_str())
                        .show_ui(ui, |ui| {
                            for baud in cmri::BAUDS {
                                ui.selectable_value(&mut options.serial.1, baud, readable::num::Unsigned::from(baud).as_str());
                            }
                        });

                    if serial_port.response.clicked() || serial_baud.response.clicked() {
                        options.sort = Sort::Serial;
                    }
                    ui.end_row();

                    ui.radio_value(&mut options.sort, Sort::Network, "Network (TCP)");
                    if ui.text_edit_singleline(&mut options.network).clicked() {
                        options.sort = Sort::Network;
                    }
                    ui.end_row();

                    extra_content(ui);
                });
            },
            |modal, ui| {
                if modal.suggested_button(ui, "Connect").clicked() {
                    *connect = true;
                }
                if let Some(cancel) = cancel {
                    *cancel = modal.caution_button(ui, "Cancel").clicked();
                }
            }
        );
    }

    fn connect(mut options: Options, modal: &egui_modal::Modal, tokio_handle: tokio::runtime::Handle) -> std::thread::JoinHandle<Result<Connection, (Options, std::io::Error)>> {
        modal.show(|ui| {
            modal.title(ui, "Connecting");
            modal.body_and_icon(ui, "Connecting.", egui_modal::Icon::Info);
        });
        modal.open();

        std::thread::spawn(move || {
            tokio_handle.block_on(async move {
                options.try_connect().map_err(|error| (options, error))
            })
        })
    }
}

impl Default for State {
    fn default() -> Self {
        Self::Prompting((Options::default(), false))
    }
}
