use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::error;
use cmri::{Address, packet::{Data, Packet, Payload}, NodeSort};
use cmri_tools::{connection::Connection, file};


#[derive(Eq, PartialEq)]
pub struct Node {
    pub(crate) address: Address,
    pub(crate) name: Option<String>,
    pub(crate) sort: Option<NodeSort>,
    pub(crate) labels: file::Labels,
    pub(crate) inputs: Data,
    pub(crate) outputs: Data
}

impl Node {
    /// # Panics
    ///
    /// If address is not betwen 0 and 127 (inclusive)
    #[expect(clippy::unwrap_used)]
    #[must_use]
    pub fn new(address: u8) -> Self {
        Self {
            address: Address::try_from_node_address(address).unwrap(),
            name: None,
            sort: None,
            labels: file::Labels::default(),
            inputs: Data::default(),
            outputs: Data::default()
        }
    }
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
         .field("name", &self.name)
         .field("address", &self.address)
         .field("sort", &self.sort)
         .field("labels", &self.labels)
         .field("inputs", &self.inputs.as_slice())
         .field("outputs", &self.outputs.as_slice())
         .finish()
    }
}


pub struct State {
    pub(crate) nodes: [Node; 128],
    pub(crate) egui_ctx: egui::Context
}

impl State {
    /// Reset the state back to default.
    pub fn reset(&mut self) {
        for node in &mut self.nodes {
            node.name = None;
            node.sort = None;
        }
    }

    pub fn load_nodes(&mut self, mut nodes: Vec<Option<file::Node>>) {
        for node in nodes.iter_mut().filter_map(Option::take) {
            let index = node.address.as_node_address() as usize;
            if node.name.is_some() {
                self.nodes[index].name = node.name;
            }
            if self.nodes[index].sort.is_none() {
                self.nodes[index].sort = Some(node.sort);
            }
            self.nodes[index].labels = node.labels;
        }
    }
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
         .field("nodes", &self.nodes)
         .finish_non_exhaustive()
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            nodes: std::array::from_fn(|i| Node::new(i.try_into().expect("i will always valid as nodes.len() < usize::MAX"))),
            egui_ctx: egui::Context::default()
        }
    }
}


/// Run a connection - updating nodes with received packets and responding to poll requests.
///
/// # Panics
///
/// If anything else on the CMRInet responds to a poll requst.
#[expect(clippy::significant_drop_tightening)]
pub fn run_connection(mut connection: Connection, state: Arc<Mutex<State>>, tokio_handle: &tokio::runtime::Handle) -> tokio::task::JoinHandle<std::io::Result<()>> {
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
                            let mut state = state.lock().await;
                            let address = packet.address().as_node_address();
                            let node = &mut state.nodes[packet.address().as_node_address() as usize];
                            match packet.payload() {
                                Payload::Initialization { node_sort } => {
                                    node.inputs = Data::new(node_sort.configuration().input_bytes().into());
                                    node.outputs = Data::new(node_sort.configuration().output_bytes().into());
                                    node.sort = Some(*node_sort);
                                    state.egui_ctx.request_repaint();
                                },
                                Payload::PollRequest => {
                                    if node.sort.is_some() {
                                        let packet = Packet::new_receive_data(packet.address(), node.inputs);
                                        let frame = packet.encode_frame();
                                        let _ = connection.send(&frame).await;
                                    }
                                },
                                Payload::TransmitData { data } => {
                                    node.outputs = *data;
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
    })
}


#[allow(clippy::missing_panics_doc, reason="tests")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default() {
        let default = State::default();
        assert_eq!(default.nodes.len(), 128);
        for (index, node) in default.nodes.iter().enumerate() {
            assert_eq!(node.address, Address::try_from_node_address(u8::try_from(index).unwrap()).unwrap());
            assert_eq!(node.sort, None);
            assert_eq!(node.name, None);
            assert_eq!(node.inputs, Data::default());
            assert_eq!(node.outputs, Data::default());
        }
    }

    #[test]
    fn reset() {
        let mut state = State::default();
        state.nodes[5].name = Some(String::from("changed"));

        state.reset();
        assert_eq!(state.nodes, State::default().nodes);
    }

    #[test]
    fn load_nodes() {
        let sort = cmri::NodeSort::try_new_smini(0, [0; 6]).unwrap();
        let nodes = vec![
            Some(file::Node {
                address: Address::try_from_node_address(10).unwrap(),
                name: Some(String::from("Test node 1")),
                sort: cmri::NodeSort::try_new_smini(0, [0; 6]).unwrap(),
                labels: file::Labels::default()
            }),
            Some(file::Node {
                address: Address::try_from_node_address(20).unwrap(),
                name: Some(String::from("Test node 2")),
                sort: cmri::NodeSort::try_new_smini(0, [3; 6]).unwrap(), // Must be different to sort variable
                labels: file::Labels::default()
            })
        ];
        let mut state = State::default();
        state.nodes[10].inputs = cmri::packet::Data::try_from(&[1]).unwrap();
        state.nodes[10].outputs = cmri::packet::Data::try_from(&[2]).unwrap();
        state.nodes[20].sort = Some(sort);
        state.load_nodes(nodes);

        assert_eq!(state.nodes[10].name, Some(String::from("Test node 1")));
        assert_eq!(state.nodes[10].sort, Some(sort));
        assert_eq!(state.nodes[10].inputs.as_slice(), [1].as_slice());  // Should be untouched
        assert_eq!(state.nodes[10].outputs.as_slice(), [2].as_slice()); // Should be untouched

        assert_eq!(state.nodes[20].name, Some(String::from("Test node 2")));
        assert_eq!(state.nodes[20].sort, Some(sort)); // Should not be replaced as it was present
    }
}
