//! Monitor a CMRInet network; gathering statistics, present nodes, and node states.

use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::error;
use cmri_tools::connection::Connection;

mod node;
mod state;
mod statistics;

pub use node::Node;
pub use state::State;
pub use statistics::Statistics;

pub fn run_connection(mut connection: Connection, state: Arc<Mutex<State>>, tokio_handle: &tokio::runtime::Handle) -> tokio::task::JoinHandle<std::io::Result<()>> {
    tokio_handle.spawn(async move {
        loop {
            match connection.receive().await {
                Err(error) => {
                    error!("Read error: {error}");
                    break Err(error);
                },
                Ok(frame) => {
                    match frame.try_as_packet() {
                        Err(error) => {
                            error!("Bad packet: {error:?}");
                            state.lock().await.got_bad_packet(frame.address());
                        },
                        Ok(packet) => {
                            state.lock().await.got_packet(&packet);
                        }
                    }
                }
            }
        }
    })
}

pub fn run_ticker(state: Arc<Mutex<State>>, tokio_handle: &tokio::runtime::Handle) -> tokio::task::JoinHandle<anyhow::Result<()>> {
    tokio_handle.spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        interval.tick().await; // Disregard the first tick as it's immediate
        loop {
            interval.tick().await;
            state.lock().await.tick();
        }
    })
}


#[allow(clippy::missing_panics_doc, reason="tests")]
#[cfg(test)]
mod tests {
    use super::*;

    mod run_connection {
        use super::*;

        #[tokio::test]
        #[allow(clippy::significant_drop_tightening)]
        async fn receives_packets() {
            let connection = tokio_test::io::Builder::new()
                // Two poll requests - one each for nodes 0 and 1
                .read(&[0xFF, 0xFF, 0x02, 65, b'P', 0x03, 0xFF, 0xFF, 0x02, 66, b'P', 0x03])
                .build();
            let connection = Connection::new("test connection", Box::new(connection));
            let state = Arc::new(Mutex::new(State::default()));
            run_connection(connection, state.clone(), &tokio::runtime::Handle::current());

            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            let state = state.lock().await;
            assert_eq!(state.statistics.poll_packets.1, 2);
            assert_eq!(state.nodes[0].statistics.poll_packets.1, 1);
            assert_eq!(state.nodes[1].statistics.poll_packets.1, 1);
        }

        #[tokio::test]
        async fn receives_bad_packets() {
            let connection = tokio_test::io::Builder::new()
                // Invalid unit address
                .read(&[0xFF, 0xFF, 0x02, b'P', 0x00, 0x03])
                // Invalid message type
                .read(&[0xFF, 0xFF, 0x02, 0x00, 65, 0x03])
                .build();
            let connection = Connection::new("test connection", Box::new(connection));
            let state = Arc::new(Mutex::new(State::default()));
            run_connection(connection, state.clone(), &tokio::runtime::Handle::current());

            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            assert_eq!(state.lock().await.statistics.bad_packets.1, 2);
        }
    }

    #[tokio::test]
    #[allow(clippy::significant_drop_tightening)]
    async fn run_ticker() {
        let state = Arc::new(Mutex::new(State::default()));
        super::run_ticker(state.clone(), &tokio::runtime::Handle::current());
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        {
            let mut state = state.lock().await;
            assert_eq!(*state, State::default());
            state.got_bad_packet(None);
        }

        // Wait for the ticker to have ticked.
        tokio::time::pause();
        tokio::time::advance(std::time::Duration::from_secs(1)).await;
        tokio::time::resume();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        {
            let state = state.lock().await;
            assert_eq!(state.statistics.bad_packets.0, 0);
            assert_eq!(state.statistics.bad_packets.1, 1);
            assert_eq!(state.statistics.bad_packets.2.len(), 1);
        }
    }
}
