//! Create and use a connection to a CMRInet network.

use tokio::io::{AsyncReadExt, AsyncWriteExt, BufStream, AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_serial::SerialStream;
use tracing::{trace, debug, info, warn};
use cmri::frame::{Raw as RawFrame, ReceiveError};

const BUFFER_LEN: usize = 128;

/// Trait for anything which can be used as a `Connection` to a CMRInet bus.
#[expect(clippy::module_name_repetitions)]
pub trait CanBeConnection: AsyncRead + AsyncWrite + std::fmt::Debug + Send + std::marker::Unpin + 'static {}
impl<T> CanBeConnection for T where T: AsyncRead + AsyncWrite + std::fmt::Debug + Send + std::marker::Unpin + 'static {}

/// A named connection to a CMRInet.
pub struct Connection {
    name: String,
    buffer: BufStream<Box<dyn CanBeConnection>>,
    frame: RawFrame
}

impl Connection {
    /// Create a new connection from a boxed stream.
    pub fn new(name: impl Into<String>, connection: Box<impl CanBeConnection>) -> Self {
        Self {
            name: name.into(),
            buffer: BufStream::with_capacity(BUFFER_LEN, BUFFER_LEN, connection),
            frame: RawFrame::new()
        }
    }

    /// Get the connection's name.
    #[must_use]
    #[allow(clippy::missing_const_for_fn, reason = "False positive: cannot perform non-const deref coercion on `std::string::String` in constant functions")]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Send a frame to the CMRInet.
    ///
    /// # Errors
    ///
    /// If the connection can't be written to, E.G.:
    /// * `std::io::ErrorKind::ConnectionReset`
    /// * `std::io::ErrorKind::ConnectionAborted`
    /// * `std::io::ErrorKind::UnexpectedEof`
    /// * `std::io::ErrorKind::HostUnreachable`
    /// * `std::io::ErrorKind::NetworkUnreachable`
    /// * `std::io::ErrorKind::NetworkDown`
    /// * `std::io::ErrorKind::BrokenPipe`
    pub async fn send(&mut self, frame: &RawFrame) -> std::io::Result<()> {
        debug!("Sending to {}: {:?}", self.name, frame);
        self.buffer.write_all(frame).await?;
        self.buffer.flush().await
    }

    /// Receive a frame from the CMRInet.
    ///
    /// # Errors
    ///
    /// If the connection can't be read from, E.G.:
    /// * `std::io::ErrorKind::ConnectionReset`
    /// * `std::io::ErrorKind::ConnectionAborted`
    /// * `std::io::ErrorKind::NetworkDown`
    /// * `std::io::ErrorKind::BrokenPipe`
    pub async fn receive(&mut self) -> std::io::Result<RawFrame> {
        loop {
            let byte = self.buffer.read_u8().await?;
            trace!("{} read byte {} {:02x}", self.name, byte, byte);
            match self.frame.receive(byte) {
                Err(ReceiveError::AlreadyComplete) => unreachable!(),
                Err(error) => warn!("Received bad frame from {}: {error:?}", self.name),
                Ok(false) => (),
                Ok(true) => {
                    debug!("Received from {}: {:?}", self.name, self.frame);
                    return Ok(std::mem::take(&mut self.frame));
                }
            }
        }
    }

    /// Shutdown/close the connection.
    ///
    /// # Errors
    ///
    /// If the connection can't be written to, E.G.:
    /// * `std::io::ErrorKind::HostUnreachable`
    /// * `std::io::ErrorKind::NetworkUnreachable`
    /// * `std::io::ErrorKind::NetworkDown`
    /// * `std::io::ErrorKind::BrokenPipe`
    pub async fn shutdown(&mut self) -> std::io::Result<()> {
        self.buffer.get_mut().shutdown().await
    }

    /// Create a new connection to a TCP server.
    ///
    /// # Errors
    ///
    /// If the connection can't be made, E.G.:
    /// * `std::io::ErrorKind::ConnectionAborted`
    /// * `std::io::ErrorKind::HostUnreachable`
    /// * `std::io::ErrorKind::NetworkUnreachable`
    /// * `std::io::ErrorKind::NetworkDown`
    pub fn new_tcp_client(address: impl Into<String>) -> std::io::Result<Self> {
        let address = address.into();
        let connection = std::net::TcpStream::connect(&address)?;
        connection.set_nonblocking(true)?;
        let connection = tokio::net::TcpStream::from_std(connection)?;
        info!("Connected to {address}");
        Ok(Self::new(address, Box::new(connection)))
    }

    /// Create a new connection to a serial port.
    ///
    /// # Errors
    ///
    /// If the connection can't be written to, E.G.:
    /// * `std::io::ErrorKind::PermissionDenied`
    /// * `std::io::ErrorKind::ResourceBusy`
    pub fn new_serial_port(port: &str, baud: u32) -> std::io::Result<Self> {
        use tokio_serial::{DataBits, StopBits, Parity, FlowControl};
        let connection = tokio_serial::new(port, baud)
            .data_bits(DataBits::Eight)
            .stop_bits(StopBits::One)
            .parity(Parity::None)
            .flow_control(FlowControl::None);
        #[allow(unused_mut)] // Only unix requires it to be mutable
        let mut connection = SerialStream::open(&connection)?;
        #[cfg(unix)]
        connection.set_exclusive(true)?;
        info!("Connected to {port} at {}bps", readable::num::Unsigned::from(baud));
        Ok(Self::new(port, Box::new(connection)))
    }
}

impl TryFrom<std::net::TcpStream> for Connection {
    type Error = std::io::Error;
    fn try_from(connection: std::net::TcpStream) -> std::io::Result<Self> {
        let name = connection.peer_addr()?.to_string();
        connection.set_nonblocking(true)?;
        let connection = TcpStream::from_std(connection)?;
        Ok(Self::new(name, Box::new(connection)))
    }
}

impl TryFrom<tokio::net::TcpStream> for Connection {
    type Error = std::io::Error;
    fn try_from(connection: tokio::net::TcpStream) -> std::io::Result<Self> {
        let name = connection.peer_addr()?.to_string();
        Ok(Self::new(name, Box::new(connection)))
    }
}

impl core::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
         .field("name", &self.name)
         .field("frame", &self.frame)
         .finish_non_exhaustive()
    }
}


/// Split a 'String' (probabbly from the commandline) into a tuple of port and speed.
///
/// # Errors
///
/// * `std::num::ParseIntError` if the baud rate (after the first ':') can't be parsed into a `u32`.
///
/// If no speed is included then `cmri::DEFAULT_BAUD` is used.
///
/// # Example
///
/// ```
/// use cmri_tools::connection::port_baud_from_str;
/// assert_eq!(port_baud_from_str("/dev/ttyACM0:9600"), Ok(("/dev/ttyACM0", 9600)));
/// assert_eq!(port_baud_from_str("/dev/ttyACM0"), Ok(("/dev/ttyACM0", cmri::DEFAULT_BAUD)));
/// assert_eq!(port_baud_from_str("/dev/ttyACM0:invalid").err().unwrap().kind(), &std::num::IntErrorKind::InvalidDigit);
/// ```
pub fn port_baud_from_str(str: &str) -> Result<(&str, u32), std::num::ParseIntError> {
    match str.split_once(':') {
        None => Ok((str, cmri::DEFAULT_BAUD)),
        Some((port, baud)) => Ok((port, baud.parse()?))
    }
}


#[allow(clippy::missing_panics_doc, reason="tests")]
#[cfg(test)]
mod tests {
    use cmri::Address;
    use super::*;

    mod port_baud_from_str {
        use super::*;

        #[test]
        fn with_path_and_baud() {
            assert_eq!(
                port_baud_from_str("/dev/ttyACM12:9600"),
                Ok(("/dev/ttyACM12", 9600))
            );
        }

        #[test]
        fn with_path_and_no_baud() {
            assert_eq!(
                port_baud_from_str("/dev/ttyACM24"),
                Ok(("/dev/ttyACM24", cmri::DEFAULT_BAUD))
            );
        }

        #[test]
        fn with_path_and_invalid_baud() {
            assert_eq!(
                port_baud_from_str("/dev/ttyACM6:invalid").err().unwrap().kind(),
                &std::num::IntErrorKind::InvalidDigit
            );
        }
    }

    mod connection {
        use cmri::packet::Packet;
        use super::*;

        mod send {
            use super::*;

            #[tokio::test]
            async fn success() {
                let frame = Packet::new_poll_request(Address::try_from_node_address(5).unwrap()).encode_frame();
                let stream = tokio_test::io::Builder::new()
                    .write(frame.as_slice())
                    .build();
                let mut connection = Connection::new("connection", Box::new(stream));
                assert!(connection.send(&frame).await.is_ok());
            }

            #[tokio::test]
            async fn failure() {
                let frame = Packet::new_poll_request(Address::try_from_node_address(5).unwrap()).encode_frame();
                let error = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "testing");
                let stream = tokio_test::io::Builder::new()
                    .write_error(error)
                    .build();
                let mut connection = Connection::new("connection", Box::new(stream));
                assert!(connection.send(&frame).await.is_err_and(|e| e.kind() == std::io::ErrorKind::BrokenPipe));
            }
        }

        mod receive {
            use super::*;

            #[tokio::test]
            async fn one_frame() {
                let stream = tokio_test::io::Builder::new()
                    .read(&[0x10, 0x11, 0x12, 0x03]) // Ending of previous frame
                    .read(&[0xFF, 0xFF, 0x02, 70])   // Start of a poll request frame
                    .read(&[b'P', 0x03, 0xFF, 0xFF]) // Finish frame and Start of next frame
                    .build();
                let mut connection = Connection::new("connection", Box::new(stream));
                assert_eq!(
                    connection.receive().await.unwrap(),
                    Packet::new_poll_request(Address::try_from_node_address(5).unwrap()).encode_frame()
                );
            }

            #[tokio::test]
            async fn two_frames() {
                let stream = tokio_test::io::Builder::new()
                    .read(&[0xFF, 0xFF, 0x02, 75, b'P', 0x03]) // Poll request for node 10
                    .read(&[0xFF, 0xFF, 0x02, 85, b'P', 0x03]) // Poll request for node 20
                    .build();
                let mut connection = Connection::new("connection", Box::new(stream));
                assert_eq!(
                    connection.receive().await.unwrap().try_as_packet().unwrap(),
                    Packet::new_poll_request(Address::try_from_node_address(10).unwrap())
                );
                assert_eq!(
                    connection.receive().await.unwrap().try_as_packet().unwrap(),
                    Packet::new_poll_request(Address::try_from_node_address(20).unwrap())
                );
            }

            #[tokio::test]
            async fn bad_frame() {
                let slice = &[0xFF, 0xFF, 0x02, 80, b'P', 0x03];
                let stream = tokio_test::io::Builder::new()
                    .read(&[0xFF, 0xFF, 0x02, 0x03]) // A too short frame - ignored
                    .read(slice)                     // Accepted and returned
                    .build();
                let mut connection = Connection::new("connection", Box::new(stream));
                assert_eq!(connection.receive().await.unwrap().as_slice(), slice);
            }

            #[tokio::test]
            async fn io_error() {
                let error = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "testing");
                let stream = tokio_test::io::Builder::new()
                    .read_error(error)
                    .build();
                let mut connection = Connection::new("connection", Box::new(stream));
                assert!(connection.receive().await.is_err_and(|e| e.kind() == std::io::ErrorKind::BrokenPipe));
            }
        }

        #[test]
        fn name() {
            let stream = tokio_test::io::Builder::new().build();
            let connection = Connection::new("connection name", Box::new(stream));
            assert_eq!(
                connection.name(),
                "connection name"
            );
        }
    }
}
