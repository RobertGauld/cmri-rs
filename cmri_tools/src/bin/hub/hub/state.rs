use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use cmri_tools::readings::Readings;
use super::{Hub, SubscriberMessage};

const READINGS_SIZE: usize = 300; // 5 minutes worth

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
#[expect(clippy::module_name_repetitions)]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Errored(String)
}

#[derive(Debug)]
pub struct State {
    frames: (u16, u64, Readings<u16, READINGS_SIZE>),  // (current second, total, previous READINGS_SIZE)
    bytes: (u32, u64, Readings<u32, READINGS_SIZE>),   // (current second, total, previous READINGS_SIZE)
    connections: HashMap<String, ConnectionState>,
    server: Option<String>
}

impl State {
    pub(super) async fn new(hub: &Hub) -> Arc<Mutex<Self>> {
        let state = Arc::new(Mutex::new(
            Self {
                frames: (0, 0, Readings::new()),
                bytes: (0, 0, Readings::new()),
                connections: HashMap::new(),
                server: None
            }
        ));

        Self::run_receiver(
            state.clone(),
            hub.subscribe(String::from("State monitor")).await
        );

        Self::run_ticker(state.clone());
        state
    }

    /// The currently connected peers.
    pub fn connections(&self) -> std::collections::hash_map::Iter<'_, String, ConnectionState> {
        self.connections.iter()
    }

    /// The address of the server, if it's running.
    pub const fn server(&self) -> Option<&String> {
        self.server.as_ref()
    }

    /// The number of frames which the `Hub` has handled.
    /// (current second, total, previous `READINGS_SIZE`)
    pub const fn frames(&self) -> &(u16, u64, Readings<u16, READINGS_SIZE>) {
        &self.frames
    }

    /// The number of bytes which the `Hub` has handled.
    /// (current second, total, previous `READINGS_SIZE`)
    pub const fn bytes(&self) -> &(u32, u64, Readings<u32, READINGS_SIZE>) {
        &self.bytes
    }

    /// Run the receiver to update this `State` from event omitted by the `Hub`.
    fn run_receiver(state: Arc<Mutex<Self>>, mut receiver: super::SubscriberRx) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    None => break,
                    Some(SubscriberMessage::Connected(address)) => {
                        state.lock().await.connections.insert(address, ConnectionState::Connected);
                    },
                    Some(SubscriberMessage::Disconnected(address)) => {
                        if state.lock().await.connections.get(&address).is_some_and(|a| !matches!(a, ConnectionState::Errored(_))) {
                            state.lock().await.connections.insert(address, ConnectionState::Disconnected);
                        }
                    },
                    Some(SubscriberMessage::Errored(address, error)) => {
                        state.lock().await.connections.insert(address, ConnectionState::Errored(error));
                    },
                    Some(SubscriberMessage::ServerStarted(address)) => {
                        state.lock().await.server = Some(address);
                    }
                    #[expect(clippy::cast_possible_truncation, reason="Frame length can never exceed 518")]
                    Some(SubscriberMessage::Frame(_source, frame)) => {
                        let mut state = state.lock().await;
                        state.frames.0 += 1;
                        state.frames.1 += 1;
                        state.bytes.0 += frame.len() as u32;
                        state.bytes.1 += frame.len() as u64;
                    }
                }
            }
        })
    }

    fn run_ticker(state: Arc<Mutex<Self>>) -> tokio::task::JoinHandle<anyhow::Result<()>> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
            interval.tick().await; // Disregard the first tick as it's immediate
            loop {
                interval.tick().await;
                let mut state = state.lock().await;
                let frames = std::mem::take(&mut state.frames.0);
                state.frames.2.push(frames);
                let bytes = std::mem::take(&mut state.bytes.0);
                state.bytes.2.push(bytes);
            }
        })
    }
}


#[allow(clippy::missing_panics_doc, reason="tests")]
#[cfg(test)]
mod tests {
    use cmri::Address;
    use super::*;

    #[test]
    fn connections() {
        let mut state = State {
            frames: (0, 0, Readings::new()),
            bytes: (0, 0, Readings::new()),
            connections: HashMap::new(),
            server: None
        };

        assert_eq!(
            state.connections().collect::<Vec<(&String, &ConnectionState)>>(),
            Vec::new()
        );

        state.connections.insert(String::from("A"), ConnectionState::Connected);
        state.connections.insert(String::from("B"), ConnectionState::Disconnected);

        let mut connections = state.connections().collect::<Vec<(&String, &ConnectionState)>>();
        connections.sort();
        assert_eq!(
            connections,
            vec![
                (&String::from("A"), &ConnectionState::Connected),
                (&String::from("B"), &ConnectionState::Disconnected)
            ]
        );
    }

    mod updates_connections {
        use super::*;
        use crate::hub::SubscriberMessage;

        async fn create() -> (Hub, Arc<Mutex<State>>) {
            let (hub, state) = crate::hub::new().await;
            let mut lock = state.lock().await;
            let connections = &mut lock.connections;
            assert!(connections.is_empty());
            connections.insert(String::from("A"), ConnectionState::Connected);
            drop(lock);
            (hub, state)
        }

        #[tokio::test]
        async fn connected() {
            let (hub, state) = create().await;
            hub.publish(SubscriberMessage::Connected(String::from("B"))).await;
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            assert_eq!(state.lock().await.connections.get("B"), Some(&ConnectionState::Connected));
        }

        #[tokio::test]
        async fn errored() {
            let (hub, state) = create().await;
            hub.publish(SubscriberMessage::Errored(String::from("A"), String::from("Error"))).await;
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            assert_eq!(state.lock().await.connections.get("A"), Some(&ConnectionState::Errored(String::from("Error"))));
        }

        #[tokio::test]
        async fn disconnected() {
            let (hub, state) = create().await;
            hub.publish(SubscriberMessage::Disconnected(String::from("A"))).await;
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            assert_eq!(state.lock().await.connections.get("A"), Some(&ConnectionState::Disconnected));
        }
    }

    #[expect(clippy::significant_drop_tightening)]
    mod updates_statistics {
        use super::*;
        use cmri::packet::Packet;

        #[tokio::test]
        async fn updates_on_received_frames() {
            let (hub, state) = crate::hub::new().await;

            {
                let state = state.lock().await;
                assert_eq!(state.bytes().0, 0);  // Current second
                assert_eq!(state.bytes().1, 0);  // Total
                assert_eq!(state.frames().0, 0); // Current second
                assert_eq!(state.frames().1, 0); // Total
            }

            let frame = Packet::new_poll_request(Address::try_from_node_address(0).unwrap()).encode_frame();
            hub.publish(SubscriberMessage::Frame(String::from("test"), Arc::new(frame))).await;
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;

            {
                let state = state.lock().await;
                assert_eq!(state.bytes().0, 6);  // Current second
                assert_eq!(state.bytes().1, 6);  // Total
                assert_eq!(state.frames().0, 1); // Current second
                assert_eq!(state.frames().1, 1); // Total
            }

            let frame = Packet::new_receive_data(Address::try_from_node_address(0).unwrap(), [1, 2, 3, 4, 5, 6, 7, 8].try_into().unwrap()).encode_frame();
            hub.publish(SubscriberMessage::Frame(String::from("test"), Arc::new(frame))).await;
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;

            {
                let state = state.lock().await;
                assert_eq!(state.bytes().0, 22);  // Current second
                assert_eq!(state.bytes().1, 22);  // Total
                assert_eq!(state.frames().0, 2); // Current second
                assert_eq!(state.frames().1, 2); // Total
                assert!(state.bytes().2.as_vec().is_empty());  // No whole econds have elapsed
                assert!(state.frames().2.as_vec().is_empty()); // No whole econds have elapsed
            }
        }

        #[tokio::test]
        async fn updates_readings() {
            let (hub, state) = crate::hub::new().await;

            {
                let state = state.lock().await;
                assert_eq!(state.bytes().0, 0);  // Current second
                assert_eq!(state.bytes().1, 0);  // Total
                assert_eq!(state.frames().0, 0); // Current second
                assert_eq!(state.frames().1, 0); // Total
            }

            let frame = Packet::new_poll_request(Address::try_from_node_address(0).unwrap()).encode_frame();
            hub.publish(SubscriberMessage::Frame(String::from("test"), Arc::new(frame))).await;
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;

            {
                let state = state.lock().await;
                assert_eq!(state.bytes().0, 6);  // Current second
                assert_eq!(state.bytes().1, 6);  // Total
                assert_eq!(state.frames().0, 1); // Current second
                assert_eq!(state.frames().1, 1); // Total
            }

            // Wait for the ticker to have ticked.
            tokio::time::pause();
            tokio::time::advance(std::time::Duration::from_secs(1)).await;
            tokio::time::resume();
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;

            let frame = Packet::new_receive_data(Address::try_from_node_address(0).unwrap(), [1, 2, 3, 4, 5, 6, 7, 8].try_into().unwrap()).encode_frame();
            hub.publish(SubscriberMessage::Frame(String::from("test"), Arc::new(frame))).await;
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;

            {
                let state = state.lock().await;
                assert_eq!(state.bytes().0, 16);  // Current second
                assert_eq!(state.bytes().1, 22);  // Total
                assert_eq!(state.frames().0, 1); // Current second
                assert_eq!(state.frames().1, 2); // Total
                assert_eq!(state.bytes().2.as_vec(), vec![6_u32]);
                assert_eq!(state.frames().2.as_vec(), vec![1_u16]);
            }
        }
    }
}
