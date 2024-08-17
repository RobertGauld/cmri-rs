use tracing::{warn, debug};
use cmri::{Address, packet::{Packet, Payload, Data}, NodeSort};
use cmri_tools::file;
use super::Statistics;

/// What's known about a node on the CMRInet network.
#[derive(Eq, PartialEq)]
pub struct Node {
    pub(super) name: Option<String>,
    pub(super) address: Address,
    pub(super) sort: Option<NodeSort>,
    pub(super) labels: file::Labels,
    pub(super) inputs: Option<Data>,
    pub(super) outputs: Option<Data>,
    pub(super) initialization_count: u16,
    pub(super) statistics: Statistics
}

impl Node {
    /// # Panics
    ///
    /// If address is not between 0 and 127 (inclusive)
    #[expect(clippy::unwrap_used)]
    #[must_use]
    pub fn new(address: u8) -> Self {
        Self {
            address: Address::try_from_node_address(address).unwrap(),
            name: None,
            sort: None,
            labels: file::Labels::default(),
            inputs: None,
            outputs: None,
            initialization_count: 0,
            statistics: Statistics::new()
        }
    }

    /// Whether the Node has received any packets.
    #[must_use]
    pub const fn has_been_seen(&self) -> bool {
        self.statistics.packets().0 > 0
    }

    /// The friendly name of the node.
    #[must_use]
    pub const fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }

    /// The address of the node.
    #[must_use]
    pub const fn address(&self) -> &Address {
        &self.address
    }

    /// What sort of node this is (according to the last Initialization packet seen).
    #[must_use]
    pub const fn sort(&self) -> Option<&NodeSort> {
        self.sort.as_ref()
    }

    /// Labels for the node's inputs and outputs.
    #[must_use]
    pub const fn labels(&self) -> &file::Labels {
        &self.labels
    }

    /// The node's inputs.
    #[must_use]
    pub const fn inputs(&self) -> Option<&Data> {
        self.inputs.as_ref()
    }

    /// The node's outputs.
    #[must_use]
    pub const fn outputs(&self) -> Option<&Data> {
        self.outputs.as_ref()
    }

    /// How many times the node has been initialized.
    /// This is probabbly how many times the controller has deemed this node to have timed out when being polled.
    #[must_use]
    pub const fn initialization_count(&self) -> u16 {
        self.initialization_count
    }

    /// Get a reference to the CMRInet network Statistics for the node.
    #[must_use]
    pub const fn statistics(&self) -> &Statistics {
        &self.statistics
    }

    pub(super) fn got_packet(&mut self, packet: &Packet) {
        if packet.address() != self.address {
            warn!("I'm node {} but was given a packet for {}!", self.address, packet.address());
            return
        }

        self.statistics.got_packet(packet);

        match packet.payload() {
            Payload::Initialization { node_sort } => {
                debug!("Initialize {} {:?}", self.address, node_sort);
                self.initialization_count += 1;
                self.sort = Some(*node_sort);
            },
            Payload::PollRequest => {
                debug!("Poll Request {}", self.address);
            },
            Payload::ReceiveData { data } => {
                debug!("Receive data {} {:?}", self.address, data.as_slice());
                if !data.is_empty() { // Empty indicates no change
                    self.inputs = Some(*data);
                }
            },
            Payload::TransmitData { data } => {
                debug!("Transmit data {} {:?}", self.address, data.as_slice());
                self.outputs = Some(*data);
            },
            #[cfg(feature = "experimenter")]
            Payload::Unknown { .. } => ()
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
         .field("initialization_count", &self.initialization_count)
         .field("statistics", &self.statistics)
         .finish()
    }
}

impl TryFrom<&Node> for file::Node {
    type Error = ();
    fn try_from(value: &Node) -> Result<Self, Self::Error> {
        if value.sort.is_none() { return Err(()) }
        Ok(Self {
            name: value.name.clone(),
            address: value.address,
            sort: value.sort.expect("Already returned Err if it's not Some"),
            labels: value.labels.clone()
        })
    }
}


#[allow(clippy::missing_panics_doc, reason="tests")]
#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default, Debug, Eq, PartialEq)]
    struct ReducedStatistics {
        packets: u16,
        bad_packets: u16,
        initialization_packets: u16,
        poll_packets: u16,
        receive_data_packets: u16,
        transmit_data_packets: u16
    }
    impl ReducedStatistics {
        const fn new(statistics: &Statistics) -> Self {
            Self {
                packets: statistics.packets.0,
                bad_packets: statistics.bad_packets.0,
                initialization_packets: statistics.initialization_packets.0,
                poll_packets: statistics.poll_packets.0,
                receive_data_packets: statistics.receive_data_packets.0,
                transmit_data_packets: statistics.transmit_data_packets.0
            }
        }
    }

    mod got_packet {
        use super::*;

        #[test]
        fn initialization() {
            let mut node = Node::new(25);
            let sort = cmri::NodeSort::try_new_smini(0, [0; 6]).unwrap();
            let packet = Packet::new_initialization(Address::try_from_node_address(25).unwrap(), sort);
            assert_eq!(ReducedStatistics::new(node.statistics()), ReducedStatistics::default());
            assert!(node.sort.is_none());

            node.got_packet(&packet);
            assert_eq!(ReducedStatistics::new(node.statistics()), ReducedStatistics { packets: 1, initialization_packets: 1, ..Default::default() } );
            assert_eq!(node.sort, Some(sort));
        }

        #[test]
        fn poll() {
            let mut node = Node::new(25);
            let packet = Packet::new_poll_request(Address::try_from_node_address(25).unwrap());
            assert_eq!(ReducedStatistics::new(node.statistics()), ReducedStatistics::default());
            assert!(node.sort.is_none());

            node.got_packet(&packet);
            assert_eq!(ReducedStatistics::new(node.statistics()), ReducedStatistics { packets: 1, poll_packets: 1, ..Default::default() } );
        }

        mod receive_data {
            use super::*;

            #[test]
            fn with_data() {
                let mut node = Node::new(25);
                let data = [4, 5, 6];
                let packet = Packet::new_receive_data(Address::try_from_node_address(25).unwrap(), data.try_into().unwrap());
                assert_eq!(ReducedStatistics::new(node.statistics()), ReducedStatistics::default());
                assert!(node.sort.is_none());

                node.got_packet(&packet);
                assert_eq!(ReducedStatistics::new(node.statistics()), ReducedStatistics { packets: 1, receive_data_packets: 1, ..Default::default() } );
                assert!(node.inputs.is_some_and(|data| data.as_slice() == data.as_slice()));
            }

            #[test]
            fn without_data() {
                let mut node = Node::new(25);
                let data = [4, 5, 6];
                node.inputs = Some(cmri::packet::Data::try_from(&data).unwrap());
                let packet = Packet::new_receive_data(Address::try_from_node_address(25).unwrap(), [].try_into().unwrap());
                assert_eq!(ReducedStatistics::new(node.statistics()), ReducedStatistics::default());
                assert!(node.sort.is_none());

                node.got_packet(&packet);
                assert_eq!(ReducedStatistics::new(node.statistics()), ReducedStatistics { packets: 1, receive_data_packets: 1, ..Default::default() } );
                assert!(node.inputs.is_some_and(|data| data.as_slice() == data.as_slice()));
            }
        }

        #[test]
        fn transmit_data() {
            let mut node = Node::new(25);
            let data = [7, 8, 9];
            let packet = Packet::new_transmit_data(Address::try_from_node_address(25).unwrap(), data.try_into().unwrap());
            assert_eq!(ReducedStatistics::new(node.statistics()), ReducedStatistics::default());
            assert!(node.sort.is_none());

            node.got_packet(&packet);
            assert_eq!(ReducedStatistics::new(node.statistics()), ReducedStatistics { packets: 1, transmit_data_packets: 1, ..Default::default() } );
            assert!(node.outputs.is_some_and(|data| data.as_slice() == data.as_slice()));
    }

        #[test]
        fn for_different_node() {
            let mut node = Node::new(25);
            let packet = Packet::new_poll_request(Address::try_from_node_address(0).unwrap());
            assert_eq!(ReducedStatistics::new(node.statistics()), ReducedStatistics::default());

            node.got_packet(&packet);
            assert_eq!(ReducedStatistics::new(node.statistics()), ReducedStatistics::default());
        }
    }
}
