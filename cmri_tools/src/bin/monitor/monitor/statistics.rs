use cmri::packet::{Packet, Payload};
use cmri_tools::readings::Readings;

pub const READINGS_SIZE: usize = 300; // 5 minutes worth

/// CMRInet network statistics for a connection/node.
#[derive(Default, Debug, Eq, PartialEq)]
pub struct Statistics {
    pub(super) packets: (u16, u64, Readings<u16, READINGS_SIZE>),                  // (current second, total, previous READINGS_SIZE)
    pub(super) bad_packets: (u16, u64, Readings<u16, READINGS_SIZE>),              // (current second, total, previous READINGS_SIZE)
    pub(super) initialization_packets: (u16, u64, Readings<u16, READINGS_SIZE>),   // (current second, total, previous READINGS_SIZE)
    pub(super) poll_packets: (u16, u64, Readings<u16, READINGS_SIZE>),             // (current second, total, previous READINGS_SIZE)
    pub(super) receive_data_packets: (u16, u64, Readings<u16, READINGS_SIZE>),     // (current second, total, previous READINGS_SIZE)
    pub(super) transmit_data_packets: (u16, u64, Readings<u16, READINGS_SIZE>),    // (current second, total, previous READINGS_SIZE)
    #[cfg(feature = "experimenter")]
    pub(super) unknown_packets: (u16, u64, Readings<u16, READINGS_SIZE>)           // (current second, total, previous READINGS_SIZE)
}

impl Statistics {
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// How wany total packets have been seen.
    /// (current second, total, previous `READINGS_SIZE`)
    #[must_use]
    pub const fn packets(&self) -> &(u16, u64, Readings<u16, READINGS_SIZE>) {
        &self.packets
    }

    /// How many bad packets have been seen.
    /// (current second, total, previous `READINGS_SIZE`)
    #[must_use]
    pub const fn bad_packets(&self) -> &(u16, u64, Readings<u16, READINGS_SIZE>) {
        &self.bad_packets
    }

    /// How many initialization packets have been seen.
    /// (current second, total, previous `READINGS_SIZE`)
    #[must_use]
    pub const fn initialization_packets(&self) -> &(u16, u64, Readings<u16, READINGS_SIZE>) {
        &self.initialization_packets
    }

    /// How many poll request packets have been seen.
    /// (current second, total, previous `READINGS_SIZE`)
    #[must_use]
    pub const fn poll_packets(&self) -> &(u16, u64, Readings<u16, READINGS_SIZE>) {
        &self.poll_packets
    }

    /// How many receive data packets have been seen.
    ///
    /// These are sent by nodes to the controller to report input states in response to a poll request.
    /// (current second, total, previous `READINGS_SIZE`)
    #[must_use]
    pub const fn receive_data_packets(&self) -> &(u16, u64, Readings<u16, READINGS_SIZE>) {
        &self.receive_data_packets
    }

    /// How many transmit data packets have been seen.
    ///
    /// These are sent by the controller to a node to set output states.
    /// (current second, total, previous `READINGS_SIZE`)
    #[must_use]
    pub const fn transmit_data_packets(&self) -> &(u16, u64, Readings<u16, READINGS_SIZE>) {
        &self.transmit_data_packets
    }

    /// How many unknown packets have been seen.
    ///
    /// (current second, total, previous `READINGS_SIZE`)
    #[cfg(feature = "experimenter")]
    #[must_use]
    pub const fn unknown_packets(&self) -> &(u16, u64, Readings<u16, READINGS_SIZE>) {
        &self.unknown_packets
    }

    pub(super) fn got_bad_packet(&mut self) {
        self.packets.0 += 1;
        self.packets.1 += 1;
        self.bad_packets.0 += 1;
        self.bad_packets.1 += 1;
    }

    pub(super) fn got_packet(&mut self, packet: &Packet) {
        self.packets.0 += 1;
        self.packets.1 += 1;
        match packet.payload() {
            Payload::Initialization { .. } => {
                self.initialization_packets.0 += 1;
                self.initialization_packets.1 += 1;
            },
            Payload::PollRequest => {
                self.poll_packets.0 += 1;
                self.poll_packets.1 += 1;
            },
            Payload::ReceiveData { .. } => {
                self.receive_data_packets.0 += 1;
                self.receive_data_packets.1 += 1;
            },
            Payload::TransmitData { .. } => {
                self.transmit_data_packets.0 += 1;
                self.transmit_data_packets.1 += 1;
            },
            #[cfg(feature = "experimenter")]
            Payload::Unknown { .. } => {
                self.unknown_packets.0 += 1;
                self.unknown_packets.1 += 1;
            }
        }
    }

    pub(super) fn tick(&mut self) {
        self.packets.2.push(std::mem::take(&mut self.packets.0));
        self.bad_packets.2.push(std::mem::take(&mut self.bad_packets.0));
        self.initialization_packets.2.push(std::mem::take(&mut self.initialization_packets.0));
        self.poll_packets.2.push(std::mem::take(&mut self.poll_packets.0));
        self.receive_data_packets.2.push(std::mem::take(&mut self.receive_data_packets.0));
        self.transmit_data_packets.2.push(std::mem::take(&mut self.transmit_data_packets.0));
        #[cfg(feature = "experimenter")]
        self.unknown_packets.2.push(std::mem::take(&mut self.unknown_packets.0));
    }

}


#[allow(clippy::missing_panics_doc, reason="tests")]
#[cfg(test)]
mod tests {
    use cmri::Address;
    use super::*;

    #[test]
    fn got_bad_packet() {
        let new = Statistics::new();
        let mut statistics = Statistics::new();

        statistics.got_bad_packet();

        assert_eq!(
            statistics,
            Statistics {
                bad_packets: (1, 1, new.bad_packets.2),
                packets: (1, 1, new.packets.2),
                ..new
            }
        );
    }

    mod got_packet {
        use super::*;

        #[test]
        fn initialization() {
            let packet = Packet::new_initialization(Address::try_from_node_address(0).unwrap(), cmri::NodeSort::try_new_smini(0, [0; 6]).unwrap());
            let new = Statistics::new();
            let mut statistics = Statistics::new();

            statistics.got_packet(&packet);

            assert_eq!(
                statistics,
                Statistics {
                    initialization_packets: (1, 1, new.initialization_packets.2),
                    packets: (1, 1, new.packets.2),
                    ..new
                }
            );
        }

        #[test]
        fn poll() {
            let packet = Packet::new_poll_request(Address::try_from_node_address(0).unwrap());
            let new = Statistics::new();
            let mut statistics = Statistics::new();

            statistics.got_packet(&packet);

            assert_eq!(
                statistics,
                Statistics {
                    poll_packets: (1, 1, new.poll_packets.2),
                    packets: (1, 1, new.packets.2),
                    ..new
                }
            );
        }

        #[test]
        fn receive() {
            let packet = Packet::new_receive_data(Address::try_from_node_address(0).unwrap(), [0].try_into().unwrap());
            let new = Statistics::new();
            let mut statistics = Statistics::new();

            statistics.got_packet(&packet);

            assert_eq!(
                statistics,
                Statistics {
                    receive_data_packets: (1, 1, new.receive_data_packets.2),
                    packets: (1, 1, new.packets.2),
                    ..new
                }
            );
        }

        #[test]
        fn transmit() {
            let packet = Packet::new_transmit_data(Address::try_from_node_address(0).unwrap(), [0].try_into().unwrap());
            let new = Statistics::new();
            let mut statistics = Statistics::new();

            statistics.got_packet(&packet);

            assert_eq!(
                statistics,
                Statistics {
                    transmit_data_packets: (1, 1, new.transmit_data_packets.2),
                    packets: (1, 1, new.packets.2),
                    ..new
                }
            );
        }

        #[test]
        #[cfg(feature = "experimenter")]
        fn unknown() {
            let packet = Packet::try_new_unknown(Address::try_from_node_address(0).unwrap(), b'Z', [0].try_into().unwrap()).unwrap();
            let new = Statistics::new();
            let mut statistics = Statistics::new();

            statistics.got_packet(&packet);

            assert_eq!(
                statistics,
                Statistics {
                    unknown_packets: (1, 1, new.unknown_packets.2),
                    packets: (1, 1, new.packets.2),
                    ..new
                }
            );
        }
    }

    #[test]
    fn tick() {
        fn transform(tuple: &(u16, u64, Readings<u16, READINGS_SIZE>)) -> (u16, u64, &[u16]) {
            (tuple.0, tuple.1, tuple.2.as_slice())
        }

        let init_packet = Packet::new_initialization(Address::try_from_node_address(0).unwrap(), cmri::NodeSort::try_new_smini(0, [0; 6]).unwrap());
        let poll_packet = Packet::new_poll_request(Address::try_from_node_address(0).unwrap());
        let recv_packet = Packet::new_receive_data(Address::try_from_node_address(0).unwrap(), [0].try_into().unwrap());
        let trmt_packet = Packet::new_transmit_data(Address::try_from_node_address(0).unwrap(), [0].try_into().unwrap());
        #[cfg(feature = "experimenter")]
        let unkn_packet = Packet::try_new_unknown(Address::try_from_node_address(0).unwrap(), b'Z', [].try_into().unwrap()).unwrap();
        let mut statistics = Statistics::new();

        let send_packets = |statistics: &mut Statistics| {
            statistics.got_bad_packet();
            statistics.got_packet(&init_packet);
            statistics.got_packet(&init_packet);
            statistics.got_packet(&poll_packet);
            statistics.got_packet(&poll_packet);
            statistics.got_packet(&poll_packet);
            statistics.got_packet(&recv_packet);
            statistics.got_packet(&recv_packet);
            statistics.got_packet(&recv_packet);
            statistics.got_packet(&recv_packet);
            statistics.got_packet(&trmt_packet);
            statistics.got_packet(&trmt_packet);
            statistics.got_packet(&trmt_packet);
            statistics.got_packet(&trmt_packet);
            statistics.got_packet(&trmt_packet);
            #[cfg(feature = "experimenter")]
            statistics.got_packet(&unkn_packet);
        };

        send_packets(&mut statistics);
        // No ticks so everything should be in current and total only
        #[cfg(not(feature = "experimenter"))]
        assert_eq!(transform(statistics.packets()), (15, 15, [].as_slice()));
        #[cfg(feature = "experimenter")]
        assert_eq!(transform(statistics.packets()), (16, 16, [].as_slice()));
        assert_eq!(transform(statistics.bad_packets()), (1, 1, [].as_slice()));
        assert_eq!(transform(statistics.initialization_packets()), (2, 2, [].as_slice()));
        assert_eq!(transform(statistics.poll_packets()), (3, 3, [].as_slice()));
        assert_eq!(transform(statistics.receive_data_packets()), (4, 4, [].as_slice()));
        assert_eq!(transform(statistics.transmit_data_packets()), (5, 5, [].as_slice()));
        #[cfg(feature = "experimenter")]
        assert_eq!(transform(statistics.unknown_packets()), (1, 1, [].as_slice()));

        // Now ticked over so current should be reset and readings should grow
        statistics.tick();
        #[cfg(not(feature = "experimenter"))]
        assert_eq!(transform(statistics.packets()), (0, 15, [15].as_slice()));
        #[cfg(feature = "experimenter")]
        assert_eq!(transform(statistics.packets()), (0, 16, [16].as_slice()));
        assert_eq!(transform(statistics.bad_packets()), (0, 1, [1].as_slice()));
        assert_eq!(transform(statistics.initialization_packets()), (0, 2, [2].as_slice()));
        assert_eq!(transform(statistics.poll_packets()), (0, 3, [3].as_slice()));
        assert_eq!(transform(statistics.receive_data_packets()), (0, 4, [4].as_slice()));
        assert_eq!(transform(statistics.transmit_data_packets()), (0, 5, [5].as_slice()));
        #[cfg(feature = "experimenter")]
        assert_eq!(transform(statistics.unknown_packets()), (0, 1, [1].as_slice()));

        // Receive twice the previous packets and tick again
        send_packets(&mut statistics);
        send_packets(&mut statistics);
        statistics.tick();
        #[cfg(not(feature = "experimenter"))]
        assert_eq!(transform(statistics.packets()), (0, 45, [15, 30].as_slice()));
        #[cfg(feature = "experimenter")]
        assert_eq!(transform(statistics.packets()), (0, 48, [16, 32].as_slice()));
        assert_eq!(transform(statistics.bad_packets()), (0, 3, [1, 2].as_slice()));
        assert_eq!(transform(statistics.initialization_packets()), (0, 6, [2, 4].as_slice()));
        assert_eq!(transform(statistics.poll_packets()), (0, 9, [3, 6].as_slice()));
        assert_eq!(transform(statistics.receive_data_packets()), (0, 12, [4, 8].as_slice()));
        assert_eq!(transform(statistics.transmit_data_packets()), (0, 15, [5, 10].as_slice()));
        #[cfg(feature = "experimenter")]
        assert_eq!(transform(statistics.unknown_packets()), (0, 3, [1, 2].as_slice()));
    }
}
