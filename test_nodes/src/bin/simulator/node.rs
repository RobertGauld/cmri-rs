use crate::cmri_bus::Bus;
use rand::RngCore;
use defmt::*;
use cmri::{packet::Packet, Address, NodeSort};


/// Simulate a node on a CMRI network.
///
/// Holds that state, randomly toggles input and output bits.
/// Send the packets relating to the node's operation onto the network.
pub struct Node {
    address: Address,
    transmit_delay: u16,
    node_sort: NodeSort,
    data: Data
}

impl Node {
    pub fn new_cpmega(random: &mut impl RngCore, address: u8, transmit_delay: u16, options: cmri::node_configuration::CpmegaOptions, input_bytes: u8, output_bytes: u8) -> Self {
        let address = Address::try_from_node_address(address).unwrap();
        let node_sort = NodeSort::try_new_cpmega(transmit_delay, options, input_bytes, output_bytes).unwrap();
        let data = Data::new(random, input_bytes, output_bytes);
        Self { address, transmit_delay, node_sort, data }
    }

    pub fn new_cpnode(random: &mut impl RngCore, address: u8, transmit_delay: u16, options: cmri::node_configuration::CpnodeOptions, input_bytes: u8, output_bytes: u8) -> Self {
        let address = Address::try_from_node_address(address).unwrap();
        let node_sort = NodeSort::try_new_cpnode(transmit_delay, options, input_bytes, output_bytes).unwrap();
        let data = Data::new(random, input_bytes, output_bytes);
        Self { address, transmit_delay, node_sort, data }
    }

    pub fn new_smini(random: &mut impl RngCore, address: u8, transmit_delay: u16) -> Self {
        let address = Address::try_from_node_address(address).unwrap();
        let node_sort = NodeSort::try_new_smini(transmit_delay, [0; 6]).unwrap();
        let data = Data::new(random, 3, 6);
        Self { address, transmit_delay, node_sort, data }
    }

    pub fn new_usic(random: &mut impl RngCore, address: u8, transmit_delay: u16, input_bytes: u8, output_bytes: u8) -> Self {
        let address = Address::try_from_node_address(address).unwrap();
        let cards = Self::build_node_cards(input_bytes / 3, output_bytes / 3);
        let node_sort = NodeSort::try_new_usic(transmit_delay, &cards).unwrap();
        let data = Data::new(random, input_bytes, output_bytes);
        Self { address, transmit_delay, node_sort, data }
    }

    pub fn new_susic(random: &mut impl RngCore, address: u8, transmit_delay: u16, input_bytes: u8, output_bytes: u8) -> Self {
        let address = Address::try_from_node_address(address).unwrap();
        let cards = Self::build_node_cards(input_bytes / 4, output_bytes / 4);
        let node_sort = NodeSort::try_new_susic(transmit_delay, &cards).unwrap();
        let data = Data::new(random, input_bytes, output_bytes);
        Self { address, transmit_delay, node_sort, data }
    }

    pub fn transmit_initialization(&self, bus: &mut impl Bus) -> Result<(), embassy_rp::uart::Error> {
        let packet = Packet::new_initialization(self.address, self.node_sort);
        trace!("Initializing {}", Debug2Format(&packet));
        bus.transmit(&packet)
    }

    pub fn transmit_poll_receive(&self, bus: &mut impl Bus) -> Result<(), embassy_rp::uart::Error> {
        let packet = Packet::new_poll_request(self.address);
        trace!("Polling {}", Debug2Format(&packet));
        bus.transmit(&packet)?;

        if self.transmit_delay > 0 {
            let delay = u64::from(self.transmit_delay) * 10;
            defmt::trace!("Delaying for {}µs", delay);
            embassy_time::block_for(embassy_time::Duration::from_micros(delay))
        }

        let packet = Packet::new_receive_data(self.address, self.data.inputs().try_into().unwrap());
        trace!("Receiving {}", Debug2Format(&packet));
        bus.transmit(&packet)
    }

    pub fn transmit_transmit(&self, bus: &mut impl Bus) -> Result<(), embassy_rp::uart::Error> {
        let packet = Packet::new_transmit_data(self.address, self.data.outputs().try_into().unwrap());
        trace!("Transmitting {}", Debug2Format(&packet));
        bus.transmit(&packet)
    }

    pub fn shuffle_data(&mut self, random: &mut impl RngCore) {
        self.data.shuffle(random)
    }

    fn build_node_cards(input_cards: u8, output_cards: u8) -> [cmri::node_configuration::node_cards::NodeCard; 64] {
        let input_cards: usize = input_cards.into();
        let output_cards: usize = output_cards.into();
        if input_cards + output_cards > 64 { defmt::panic!("input_cards ({}) + output_cards ({}) must be ≤ 256", input_cards, output_cards) }

        let mut cards = [cmri::node_configuration::node_cards::NodeCard::None; 64];
        for i in 0..input_cards {
            cards[i] = cmri::node_configuration::node_cards::NodeCard::Input
        }
        for i in input_cards..(input_cards + output_cards) {
            cards[i] = cmri::node_configuration::node_cards::NodeCard::Output
        }
        cards
    }
}

/// Manages the inputs and outputs of a Node.
struct Data {
    data: [u8; 256],
    input_bytes: usize,
    output_bytes: usize
}

impl Data {
    fn new(random: &mut impl RngCore, input_bytes: u8, output_bytes: u8) -> Self {
        let input_bytes = input_bytes.into();
        let output_bytes = output_bytes.into();
        if input_bytes + output_bytes > 256 { defmt::panic!("input_bytes ({}) + output_bytes ({}) must be ≤ 256", input_bytes, output_bytes); }

        let mut data = [0; 256];
        let len = input_bytes + output_bytes;
        for i in (0..len).step_by(4) {
            let count = (len - i).min(4);
            let random = random.next_u32().to_ne_bytes();
            for j in 0..count {
                data[i + j] = random[j];
            }
        }

        Self { data, input_bytes, output_bytes }
    }

    fn len(&self) -> usize {
        self.input_bytes + self.output_bytes
    }

    fn inputs(&self) -> &[u8] {
        &self.data[..(self.input_bytes)]
    }

    fn outputs(&self) -> &[u8] {
        &self.data[(self.input_bytes)..(self.len())]
    }

    fn shuffle(&mut self, random: &mut impl RngCore) {
        let len = self.len();
        for byte in self.data[..len].iter_mut() {
            let random = random.next_u32().to_ne_bytes();
            if random[3] & 0b0011_1111 == 0 { // A 1 in n chance that this byte will change
                let map = random[0] & random[1] & random[2];
                *byte ^= map;
            }
        }
    }
}