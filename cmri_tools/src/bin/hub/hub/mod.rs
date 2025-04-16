//! Link multiple CMRInet Networks.

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Context;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, info, error};
use cmri::frame::Raw as RawFrame;
use cmri_tools::connection::Connection;

const CHANNEL_BUFFER: usize = 4;

type ConnectionMessage = Arc<RawFrame>;
type ConnectionTx = mpsc::Sender<ConnectionMessage>;
type ConnectionRx = mpsc::Receiver<ConnectionMessage>;

pub mod state;
use state::State;

pub async fn new() -> (Hub, Arc<Mutex<State>>) {
    let hub = Hub::new();
    let state = State::new(&hub).await;
    (hub, state)
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SubscriberMessage {
    Connected(String),
    Disconnected(String),
    Errored(String, String),
    ServerStarted(String),
    Frame(String, ConnectionMessage)
}
pub type SubscriberTx = mpsc::Sender<SubscriberMessage>;
pub type SubscriberRx = mpsc::Receiver<SubscriberMessage>;

/// Distributes packets between a number of CMRInet connections.
#[derive(Debug, Clone)]
pub struct Hub {
    inner: Arc<Mutex<Inner>>
}

#[derive(Debug)]
struct Inner {
    connections: HashMap<String, ConnectionTx>,
    subscriptions: Vec<(String, SubscriberTx)>
}

impl Hub {
    /// Create a new Hub.
    #[must_use]
    fn new() -> Self {
        let inner = Inner {
            connections: HashMap::new(),
            subscriptions: Vec::new()
        };
        Self { inner: Arc::new(Mutex::new(inner)) }
    }

    /// Start a server, and add incomming connections to the `Hub`.
    ///
    /// # Errors
    ///
    /// If the server can't bind or be configured, see:
    ///   * `std::net::TcpListener::bind`
    ///   * `std::net::TcpListener::set_nonblocking`
    #[expect(clippy::missing_panics_doc)]
    pub async fn start_server(&self, address: &str) -> anyhow::Result<std::net::SocketAddr> {
        debug!("Staring server on {address}");
        let listener = std::net::TcpListener::bind(address).context(format!("Staring server on {address}"))?;
        listener.set_nonblocking(true).context(format!("Staring server on {address}"))?;
        let address = listener.local_addr().context(format!("Staring server on {address}"))?;
        info!("Started server on {address}");
        self.publish(SubscriberMessage::ServerStarted(address.to_string())).await;

        let hub = self.clone();
        tokio::spawn(async move {
            let listener = TcpListener::from_std(listener).expect("A std listener to convert to a tokio listener");
            loop {
                match listener.accept().await {
                    Err(error) => error!("Couldn't get client: {error}"),
                    Ok((connection, addr)) => {
                        info!("Connection from {addr}");
                        match connection.try_into() {
                            Err(error) => error!("Couldn't run connection for {addr}: {error}"),
                            Ok(connection) => { hub.run_connection(connection); }
                        }
                    }
                }
            }
        });
        Ok(address)
    }

    /// Connect to a remote server, and add the connection to the `Hub`.
    ///
    /// # Errors
    ///
    /// If the connection can't be established or configured, see:
    ///   * `std::net::TcpStream::connect`
    ///   * `std::net::TcpStream::set_nonblocking`
    ///   * `tokio::net::TcpStream::from_std`
    pub fn add_network(&self, address: &str) -> anyhow::Result<()> {
        let connection = std::net::TcpStream::connect(address)?;
        info!("Connected to {address}");
        self.run_connection(connection.try_into()?);
        Ok(())
    }

    /// Connect to a serial port, and add the connection to the `Hub`.
    ///
    /// # Errors
    ///
    /// If the connection can't be established or configured, see:
    ///   * `tokio_serial::SerialStream::open`
    ///   * `tokio_serial::SerialStream::set_exclusive`
    pub fn add_serial_port(&self, port: &str, baud: u32) -> anyhow::Result<()> {
        let connection = tokio_serial::new(port, baud)
            .data_bits(tokio_serial::DataBits::Eight)
            .stop_bits(tokio_serial::StopBits::One)
            .parity(tokio_serial::Parity::None)
            .flow_control(tokio_serial::FlowControl::None);
        #[allow(unused_mut)] // Only unix requires it to be mutable
        let mut connection = tokio_serial::SerialStream::open(&connection)?;
        #[cfg(unix)]
        connection.set_exclusive(true)?;
        info!("Connected to {port} at {}bps", readable::num::Unsigned::from(baud));
        self.run_connection(Connection::new(port, Box::new(connection)));
        Ok(())
    }

    /// Receive updates from `Hub`.
    #[must_use]
    pub async fn subscribe(&self, name: String) -> SubscriberRx {
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER);
        self.inner.lock().await.subscriptions.push((name, tx));
        rx
    }

    async fn connect(&self, name: String) -> ConnectionRx {
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER);
        self.inner.lock().await.connections.insert(name.clone(), tx);
        self.publish(SubscriberMessage::Connected(name)).await;
        rx
    }

    async fn disconnect(&self, name: String) {
        self.inner.lock().await.connections.remove(&name);
        self.publish(SubscriberMessage::Disconnected(name)).await;
    }

    async fn errored(&self, name: String, error: String) {
        self.publish(SubscriberMessage::Errored(name, error)).await;
    }

    async fn broadcast(&self, source: String, message: ConnectionMessage) {
        debug!("Broadcasting {message:?}");

        // Send message to connections (except the one which received it).
        let mut inner = self.inner.lock().await;
        for (destination, channel) in &mut inner.connections {
            if destination != &source {
                if let Err(error) = channel.send(message.clone()).await {
                    error!("Couldn't enque for connection {:?}: {}", destination, error);
                }
            }
        }
        drop(inner);

        self.publish(SubscriberMessage::Frame(source, message)).await;
    }

    async fn publish(&self, message: SubscriberMessage) {
        debug!("Publishing {message:?}");
        let mut inner = self.inner.lock().await;
        for (destination, channel) in &mut inner.subscriptions {
            if let Err(error) = channel.send(message.clone()).await {
                error!("Couldn't enque for subscriber {:?}: {}", destination, error);
            }
        }
    }

    /// Run a connection as a tokio task.
    pub fn run_connection(&self, mut connection: Connection) -> tokio::task::JoinHandle<std::io::Result<()>> {
        let hub = self.clone();
        tokio::spawn(async move {
            let name = connection.name().to_string();
            let mut rx = hub.connect(name.clone()).await;
            let result = loop {
                tokio::select! {
                    message = rx.recv() => match message {
                        None => {
                            // Channel was closed
                            break Ok(())
                        },
                        Some(frame) => {
                            debug!("Sending {frame:?} to {}", name);
                            if let Err(error) = connection.send(&frame).await {
                                error!("Write error on {name}: {error}");
                                if matches!(error.kind(), std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted | std::io::ErrorKind::UnexpectedEof) {
                                    // Peer disconnected
                                    break Ok(())
                                }
                                hub.errored(name.to_string(), error.to_string()).await;
                            }
                        }
                    },
                    a = connection.receive() => match a {
                        Err(error) => {
                            if matches!(error.kind(), std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted | std::io::ErrorKind::UnexpectedEof) {
                                // Peer disconnected
                                break Ok(())
                            }
                            error!("Read error on {name}: {error}");
                            break Err(error);
                        },
                        Ok(frame) => {
                            debug!("Received {frame:?} from {name}");
                            let message = Arc::new(frame);
                            hub.broadcast(name.clone(), message).await;
                        }
                    }
                }
            };

            if let Err(ref error) = result {
                hub.errored(name.to_string(), error.to_string()).await;
            }

            hub.disconnect(name.to_string()).await;
            result
        })
    }
}


#[allow(clippy::missing_panics_doc, reason="tests")]
#[cfg(test)]
mod tests {
    use cmri::{Address, packet::Packet};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use super::*;

    #[tokio::test]
    async fn tcp() {
        // start server, make 2 connections and test a frame moves each way.

        let hub = Hub::new();
        let address = hub.start_server("localhost:0").await.unwrap();

        let mut connections = [
            tokio::net::TcpStream::connect(&address).await.unwrap(),
            tokio::net::TcpStream::connect(&address).await.unwrap()
        ];

        let mut buffer = [0; 8];
        let frames = [
            Packet::new_poll_request(Address::try_from_node_address(10).unwrap()).encode_frame(),
            Packet::new_poll_request(Address::try_from_node_address(20).unwrap()).encode_frame()
        ];

        assert!(connections[0].try_read(&mut buffer).is_err_and(|e| e.kind() == std::io::ErrorKind::WouldBlock ));
        assert!(connections[0].write(&frames[0]).await.is_ok_and(|u| u == frames[0].len()));
        assert!(connections[1].read(&mut buffer).await.is_ok_and(|u| { let len = frames[0].len(); u == len && &buffer[..len] == frames[0].as_slice() }));

        assert!(connections[1].try_read(&mut buffer).is_err_and(|e| e.kind() == std::io::ErrorKind::WouldBlock ));
        assert!(connections[1].write(&frames[1]).await.is_ok_and(|u| u == frames[1].len()));
        assert!(connections[0].read(&mut buffer).await.is_ok_and(|u| { let len = frames[1].len(); u == len && &buffer[..len] == frames[1].as_slice() }));
    }

    mod run_connection {
        use super::*;

        #[tokio::test]
        async fn receives_and_sends() {
            let frame_rx = Packet::new_poll_request(Address::try_from_node_address(50).unwrap()).encode_frame();
            let frame_tx = Packet::new_poll_request(Address::try_from_node_address(60).unwrap()).encode_frame();

            let connection = tokio_test::io::Builder::new()
                .read(frame_rx.as_slice())
                .write(frame_tx.as_slice())
                .build();
            let hub = Hub::new();
            let mut rx = hub.subscribe(String::from("subscriber")).await;
            hub.run_connection(Connection::new("connection", Box::new(connection)));

            // Publishes connected
            assert_eq!(rx.recv().await, Some(SubscriberMessage::Connected(String::from("connection"))));

            // Publishes frame
            assert_eq!(rx.recv().await, Some(SubscriberMessage::Frame(String::from("connection"), Arc::new(frame_rx))));

            // Sends frame
            hub.broadcast(String::from("test"), Arc::new(frame_tx)).await;
            assert!(matches!(rx.recv().await, Some(SubscriberMessage::Frame(_, _))));

            // Publishes disconnected
            assert_eq!(rx.recv().await, Some(SubscriberMessage::Disconnected(String::from("connection"))));
        }

        #[tokio::test]
        async fn write_error() {
            let frame = Packet::new_poll_request(Address::try_from_node_address(70).unwrap()).encode_frame();
            let connection = tokio_test::io::Builder::new()
                .write_error(std::io::Error::other("error"))
                .build();
            let hub = Hub::new();
            let mut rx = hub.subscribe(String::from("subscriber")).await;
            hub.run_connection(Connection::new("connection", Box::new(connection)));

            assert_eq!(rx.recv().await, Some(SubscriberMessage::Connected(String::from("connection"))));
            hub.broadcast(String::from("test"), Arc::new(frame)).await;
            assert!(matches!(rx.recv().await, Some(SubscriberMessage::Frame(_, _))));

            // Publishes the error
            assert_eq!(rx.recv().await, Some(SubscriberMessage::Errored(String::from("connection"), String::from("error"))));
        }

        #[tokio::test]
        async fn read_error() {
            let connection = tokio_test::io::Builder::new()
                .read_error(std::io::Error::other("error"))
                .build();
            let hub = Hub::new();
            let mut rx = hub.subscribe(String::from("subscriber")).await;
            hub.run_connection(Connection::new("connection", Box::new(connection)));

            assert_eq!(rx.recv().await, Some(SubscriberMessage::Connected(String::from("connection"))));

            // Publishes the error
            assert_eq!(rx.recv().await, Some(SubscriberMessage::Errored(String::from("connection"), String::from("error"))));
        }

        #[tokio::test]
        async fn bad_frame() {
            let frame = Packet::new_poll_request(Address::try_from_node_address(80).unwrap()).encode_frame();
            let connection = tokio_test::io::Builder::new()
                .read(&[0xFF, 0xFF, 0x02, 0x03])
                .read(frame.as_slice())
                .build();
            let hub = Hub::new();
            let mut rx = hub.subscribe(String::from("subscriber")).await;
            hub.run_connection(Connection::new("connection", Box::new(connection)));

            assert_eq!(rx.recv().await, Some(SubscriberMessage::Connected(String::from("connection"))));

            // Bad frame is silently ignored, so the following good one is seen next
            assert_eq!(rx.recv().await, Some(SubscriberMessage::Frame(String::from("connection"), Arc::new(frame))));
        }
    }
}
