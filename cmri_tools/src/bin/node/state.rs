use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::error;
use cmri::{Address, packet::{Packet, Payload, Data}};
use cmri_tools::connection::Connection;

pub struct State {
    pub(crate) inputs: Data,
    pub(crate) outputs: Data,
    pub(crate) initialised: bool,
    pub(crate) egui_ctx: egui::Context
}

impl State {
    pub fn new() -> Self {
        Self {
            inputs: Data::default(),
            outputs: Data::default(),
            initialised: false,
            egui_ctx: egui::Context::default()
        }
    }

    pub fn initialise(&mut self, input_bytes: usize) {
        self.inputs = Data::new(input_bytes);
        self.outputs = Data::default();
        self.initialised = true;
    }
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
         .field("inputs", &self.inputs)
         .field("outputs", &self.outputs)
         .field("initialised", &self.initialised)
         .finish_non_exhaustive()
    }
}

/// Run a connection - updating node with received packets and responding to poll requests.
///
/// # Panics
///
/// If anything else on the CMRInet responds to a poll requst.
#[expect(clippy::significant_drop_tightening)]
pub fn run_connection(state: Arc<Mutex<State>>, mut connection: Connection, address: Address, tokio_handle: &tokio::runtime::Handle) -> tokio::task::JoinHandle<std::io::Result<()>> {
    tokio_handle.spawn(async move {
        loop {
            match connection.receive().await {
                Err(error) => {
                    error!("Read error: {error}");
                    std::process::exit(1);
                },
                Ok(frame) => {
                    match frame.try_as_packet() {
                        Err(error) => error!("Bad packet: {error:?}"),
                        Ok(packet) => {
                            if packet.address() == address {
                                let mut state = state.lock().await;
                                match packet.payload() {
                                    Payload::Initialization { node_sort } => {
                                        let configuration = node_sort.configuration();
                                        state.initialise(configuration.input_bytes().into());
                                        state.egui_ctx.request_repaint();
                                    },
                                    Payload::PollRequest => {
                                        if state.initialised {
                                            let packet = Packet::new_receive_data(packet.address(), state.inputs);
                                            let frame = packet.encode_frame();
                                            let _ = connection.send(&frame).await;
                                        }
                                    },
                                    Payload::TransmitData { data } => {
                                        state.outputs = *data;
                                        state.egui_ctx.request_repaint();
                                    },
                                    Payload::ReceiveData { .. } => {
                                        panic!("Another node on the CMRInet has address {address}");
                                    },
                                    #[cfg(feature = "experimenter")]
                                    Payload::Unknown { .. } => ()
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}
