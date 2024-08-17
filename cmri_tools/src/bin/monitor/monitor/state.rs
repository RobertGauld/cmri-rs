use cmri::packet::Packet;
use cmri_tools::file;
use super::Node;
use super::Statistics;

/// Details about the CMRInet network's state.
#[derive(Eq, PartialEq)]
pub struct State {
    pub(super) statistics: Statistics,
    pub(super) nodes: Box<[Node; 128]>
}

impl State {
    /// Get the CMRInet network Statistics for the connection.
    #[must_use]
    pub const fn statistics(&self) -> &Statistics {
        &self.statistics
    }

    /// Get a reference to the list of `Node`s seen by the monitor.
    #[must_use]
    pub const fn nodes(&self) -> &[Node; 128] {
        &self.nodes
    }

    /// Reset the state back to default.
    #[expect(clippy::unwrap_used, clippy::missing_panics_doc, reason="i will never be invalid due to size of the nodes array")]
    pub fn reset(&mut self) {
        self.statistics = Statistics::default();
        for (i, node) in self.nodes.iter_mut().enumerate() {
            *node = Node::new(i.try_into().unwrap());
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

    pub(super) fn got_bad_packet(&mut self, node_address: Option<u8>) {
        self.statistics.got_bad_packet();
        if let Some(index) = node_address.map(usize::from) {
            self.nodes[index].statistics.got_bad_packet();
        }
    }

    pub(super) fn got_packet(&mut self, packet: &Packet) {
        self.statistics.got_packet(packet);
        self.nodes[usize::from(packet.address().as_node_address())].got_packet(packet);
    }

    pub(super) fn tick(&mut self) {
        self.statistics.tick();
        for node in self.nodes.as_mut() {
            node.statistics.tick();
        }
    }
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
         .field("statistics", &self.statistics)
         .field("nodes", &self.nodes)
         .finish()
    }
}

impl std::default::Default for State {
    fn default() -> Self {
        let mut nodes = Vec::with_capacity(128);
        for i in 0..128 {
            nodes.push(Node::new(i));
        }
        let nodes: Box<[Node; 128]> = nodes.try_into().expect("A Vec<Node> of length 128 to go into a Box<[Node; 128]>");

        Self { statistics: Statistics::default(), nodes }
    }
}


#[allow(clippy::missing_panics_doc, reason="tests")]
#[cfg(test)]
mod tests {
    use cmri::Address;
    use cmri_tools::readings::Readings;
    use super::*;

    #[test]
    fn default() {
        use super::*;
        use super::super::statistics::READINGS_SIZE;
        let default = State::default();

        let statistics = default.statistics();
        let check_statistics = |statistics: &(u16, u64, Readings<u16, READINGS_SIZE>)| {
            assert_eq!(statistics.0, 0, "Current second should default to zero");
            assert_eq!(statistics.1, 0, "Total should default to zero");
            assert!(statistics.2.is_empty(), "Readings should default to empty");
        };
        check_statistics(statistics.packets());
        check_statistics(statistics.bad_packets());
        check_statistics(statistics.initialization_packets());
        check_statistics(statistics.poll_packets());
        check_statistics(statistics.receive_data_packets());
        check_statistics(statistics.transmit_data_packets());

        assert_eq!(default.nodes.len(), 128);
        for (index, node) in default.nodes.iter().enumerate() {
            assert_eq!(node.address, Address::try_from_node_address(u8::try_from(index).unwrap()).unwrap());
            assert_eq!(node.sort, None);
            assert_eq!(node.name, None);
            assert_eq!(node.inputs, None);
            assert_eq!(node.outputs, None);
            assert_eq!(node.initialization_count, 0);
            assert_eq!(node.statistics, Statistics::default());
        }
    }

    #[test]
    fn reset() {
        let mut state = State::default();
        state.got_bad_packet(Some(0));
        state.nodes[5].name = Some(String::from("changed"));

        state.reset();
        assert_eq!(state, State::default());
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
        state.nodes[10].inputs = Some(cmri::packet::Data::try_from(&[1]).unwrap());
        state.nodes[10].outputs = Some(cmri::packet::Data::try_from(&[2]).unwrap());
        state.nodes[10].initialization_count = 3;
        state.nodes[10].statistics.got_bad_packet();
        state.nodes[20].sort = Some(sort);
        state.load_nodes(nodes);

        assert_eq!(state.nodes[10].name, Some(String::from("Test node 1")));
        assert_eq!(state.nodes[10].sort, Some(sort));
        assert_eq!(state.nodes[10].inputs.unwrap().as_slice(), [1].as_slice());  // Should be untouched
        assert_eq!(state.nodes[10].outputs.unwrap().as_slice(), [2].as_slice()); // Should be untouched
        assert_eq!(state.nodes[10].initialization_count, 3);                     // Should be untouched
        assert_eq!(state.nodes[10].statistics.bad_packets().1, 1);               // Should be untouched

        assert_eq!(state.nodes[20].name, Some(String::from("Test node 2")));
        assert_eq!(state.nodes[20].sort, Some(sort)); // Should not be replaced as it was present
    }

    mod got_packet {
        use super::*;

        #[test]
        fn initialization() {
            let sort = cmri::NodeSort::try_new_smini(0, [0; 6]).unwrap();
            let packet = Packet::new_initialization(Address::try_from_node_address(25).unwrap(), sort);
            let mut state = State::default();
            state.got_packet(&packet);

            let default = Statistics::default();
            assert_eq!(
                state.statistics,
                Statistics {
                    packets: (1, 1, default.packets.2),
                    initialization_packets: (1, 1, default.initialization_packets.2),
                    ..default
                }
            );
            let default = Statistics::default();
            assert_eq!(
                state.nodes[25].statistics,
                Statistics {
                    packets: (1, 1, default.packets.2),
                    initialization_packets: (1, 1, default.initialization_packets.2),
                    ..default
                }
            );
            assert_eq!(state.nodes[0].statistics, Statistics::default());
        }

        #[test]
        fn poll_request() {
            let packet = Packet::new_poll_request(Address::try_from_node_address(25).unwrap());
            let mut state = State::default();
            state.got_packet(&packet);

            let default = Statistics::default();
            assert_eq!(
                state.statistics,
                Statistics {
                    packets: (1, 1, default.packets.2),
                    poll_packets: (1, 1, default.poll_packets.2),
                    ..default
                }
            );
            let default = Statistics::default();
            assert_eq!(
                state.nodes[25].statistics,
                Statistics {
                    packets: (1, 1, default.packets.2),
                    poll_packets: (1, 1, default.poll_packets.2),
                    ..default
                }
            );
            assert_eq!(state.nodes[0].statistics, Statistics::default());
        }

        #[test]
        fn receive_data() {
            let packet = Packet::new_receive_data(Address::try_from_node_address(25).unwrap(), [0].try_into().unwrap());
            let mut state = State::default();
            state.got_packet(&packet);

            let default = Statistics::default();
            assert_eq!(
                state.statistics,
                Statistics {
                    packets: (1, 1, default.packets.2),
                    receive_data_packets: (1, 1, default.receive_data_packets.2),
                    ..default
                }
            );
            let default = Statistics::default();
            assert_eq!(
                state.nodes[25].statistics,
                Statistics {
                    packets: (1, 1, default.packets.2),
                    receive_data_packets: (1, 1, default.receive_data_packets.2),
                    ..default
                }
            );
            assert_eq!(state.nodes[0].statistics, Statistics::default());
        }

        #[test]
        fn transmit_data() {
            let packet = Packet::new_transmit_data(Address::try_from_node_address(25).unwrap(), [0].try_into().unwrap());
            let mut state = State::default();
            state.got_packet(&packet);

            let default = Statistics::default();
            assert_eq!(
                state.statistics,
                Statistics {
                    packets: (1, 1, default.packets.2),
                    transmit_data_packets: (1, 1, default.transmit_data_packets.2),
                    ..default
                }
            );
            let default = Statistics::default();
            assert_eq!(
                state.nodes[25].statistics,
                Statistics {
                    packets: (1, 1, default.packets.2),
                    transmit_data_packets: (1, 1, default.transmit_data_packets.2),
                    ..default
                }
            );
            assert_eq!(state.nodes[0].statistics, Statistics::default());
        }
    }

    mod got_bad_packet {
        use super::*;

        #[test]
        fn with_valid_address() {
            let mut state = State::default();

            state.got_bad_packet(Some(0));

            let default = Statistics::default();
            assert_eq!(
                state.statistics,
                Statistics {
                    bad_packets: (1, 1, default.bad_packets.2),
                    packets: (1, 1, default.packets.2),
                    ..default
                }
            );
            let default = Statistics::default();
            assert_eq!(
                state.nodes[0].statistics,
                Statistics {
                    bad_packets: (1, 1, default.bad_packets.2),
                    packets: (1, 1, default.packets.2),
                    ..default
                }
            );
            let default = Statistics::default();
            assert_eq!( state.nodes[2].statistics, default);
        }

        #[test]
        fn with_invalid_address() {
            let default = Statistics::default();
            let mut state = State::default();

            state.got_bad_packet(None);

            assert_eq!(
                state.statistics,
                Statistics {
                    bad_packets: (1, 1, default.bad_packets.2),
                    packets: (1, 1, default.packets.2),
                    ..default
                }
            );
        }
    }

    #[test]
    fn tick() {
        fn transform(tuple: &(u16, u64, Readings<u16, { super::super::statistics::READINGS_SIZE }>)) -> (u16, u64, &[u16]) {
            (tuple.0, tuple.1, tuple.2.as_slice())
        }

        let packet = Packet::new_poll_request(Address::try_from_node_address(0).unwrap());
        let mut state = State::default();
        state.got_packet(&packet);

        // No ticks so everything should be in current and total only
        assert_eq!(transform(state.statistics.packets()), (1, 1, [].as_slice()));
        assert_eq!(transform(state.statistics.bad_packets()), (0, 0, [].as_slice()));
        assert_eq!(transform(state.statistics.poll_packets()), (1, 1, [].as_slice()));
        assert_eq!(transform(state.nodes[0].statistics.packets()), (1, 1, [].as_slice()));
        assert_eq!(transform(state.nodes[0].statistics.bad_packets()), (0, 0, [].as_slice()));
        assert_eq!(transform(state.nodes[0].statistics.poll_packets()), (1, 1, [].as_slice()));
        assert_eq!(transform(state.nodes[1].statistics.packets()), (0, 0, [].as_slice()));
        assert_eq!(transform(state.nodes[1].statistics.bad_packets()), (0, 0, [].as_slice()));
        assert_eq!(transform(state.nodes[1].statistics.poll_packets()), (0, 0, [].as_slice()));

        // Now ticked over so current should be reset and readings should grow
        state.tick();
        assert_eq!(transform(state.statistics.packets()), (0, 1, [1].as_slice()));
        assert_eq!(transform(state.statistics.bad_packets()), (0, 0, [0].as_slice()));
        assert_eq!(transform(state.statistics.poll_packets()), (0, 1, [1].as_slice()));
        assert_eq!(transform(state.nodes[0].statistics.packets()), (0, 1, [1].as_slice()));
        assert_eq!(transform(state.nodes[0].statistics.bad_packets()), (0, 0, [0].as_slice()));
        assert_eq!(transform(state.nodes[0].statistics.poll_packets()), (0, 1, [1].as_slice()));
        assert_eq!(transform(state.nodes[1].statistics.packets()), (0, 0, [0].as_slice()));
        assert_eq!(transform(state.nodes[1].statistics.bad_packets()), (0, 0, [0].as_slice()));
        assert_eq!(transform(state.nodes[1].statistics.poll_packets()), (0, 0, [0].as_slice()));
    }
}
