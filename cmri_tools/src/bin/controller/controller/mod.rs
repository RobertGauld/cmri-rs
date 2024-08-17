use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{warn, error};
use cmri::packet::{Packet, Payload};
use cmri_tools::connection::Connection;

mod state;
pub use state::State;

mod node;
pub use node::Node;

const PERIOD: std::time::Duration = std::time::Duration::from_millis(250);
const TIMEOUT: std::time::Duration = std::time::Duration::from_millis(500);

/// Run a connection - updating nodes with received packets and responding to poll requests.
///
/// # Panics
///
/// If another controller node is detected.
pub fn run_connection(mut connection: Connection, state: Arc<Mutex<State>>, tokio_handle: &tokio::runtime::Handle) -> tokio::task::JoinHandle<std::io::Result<()>> {
    tokio_handle.spawn(async move {
        let handle_error = |error: &std::io::Error| {
            error!("{error:?}");
            std::process::exit(1);
        };

        let mut period = tokio::time::interval(PERIOD);
        loop {
            period.tick().await;
            #[expect(clippy::significant_drop_in_scrutinee)]
            for i in 0..128 {
                if let Some(node) = state.lock().await.nodes[i].as_mut() {
                    // Initialise if required
                    if node.to_initialise {
                        let packet = Packet::new_initialization(node.address, node.sort);
                        if let Err(error) = connection.send(&packet.encode_frame()).await { handle_error(&error); }
                    }

                    // Poll inputs
                    let packet = Packet::new_poll_request(node.address);
                    if let Err(error) = connection.send(&packet.encode_frame()).await { handle_error(&error); }
                    match tokio::time::timeout(TIMEOUT, connection.receive()).await {
                        Err(_) => {
                            warn!("Poll request to node {} timed out after {:?}.", node.address, TIMEOUT);
                            node.to_initialise = true;
                        },
                        Ok(Err(error)) => handle_error(&error),
                        Ok(Ok(frame)) => {
                            match frame.try_as_packet() {
                                Err(error) => warn!("Bad frame received: {error:?}"),
                                Ok(packet) => {
                                    if packet.address() == node.address {
                                        if let Payload::ReceiveData { data } = packet.payload() {
                                            node.to_initialise = false;
                                            node.inputs = *data;
                                        }
                                    } else {
                                        panic!("Another controller exists on the CMRInet.");
                                    }
                                }
                            }
                        }
                    }

                    // Set outputs
                    let packet = Packet::new_transmit_data(node.address, node.outputs);
                    if let Err(error) = connection.send(&packet.encode_frame()).await { handle_error(&error); }
                }
            }
            state.lock().await.egui_ctx.request_repaint();
        }
    })
}
