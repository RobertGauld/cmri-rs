#![no_std]
#![no_main]

//! Implements a CPMega with 1 input byte and 1 output byte on a Raspberry Pi Pico.
//! The low nibble of the output byte controls LEDs on pins gpio6, gpio7, gpio8, and gpio9.
//! The low nibble of the input byte is set by buttons on pins gpio14, gpio15, gpio16, and gpio17.
//! The high nibble of the output byte is copied to the high nibble on the input byte.
//! Debugging information is handled by defmt and probe-rs.
//! UART0 (gpio0 TX, gpio1 RX, 115200 baud, 8 data bits, 1 stop bit, no parity)
//! is used for the CMRInet network, gpio2 (high for TX) is available for controlling an RS485 module.
const NODE_ADDRESS: u8 = 5; // 0 - 127 (inclusive)

use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Output, Level, Pull};
use embassy_rp::uart::Uart;
use {defmt_rtt as _, panic_probe as _};
use cmri::NodeConfiguration;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    unsafe { embassy_rp::time_driver::init(); }
    let p = embassy_rp::init(Default::default());

    let inputs = (
        Input::new(p.PIN_14, Pull::Down),
        Input::new(p.PIN_15, Pull::Down),
        Input::new(p.PIN_16, Pull::Down),
        Input::new(p.PIN_17, Pull::Down)
    );

    let mut outputs = (
        Output::new(p.PIN_6, Level::Low),
        Output::new(p.PIN_7, Level::Low),
        Output::new(p.PIN_8, Level::Low),
        Output::new(p.PIN_9, Level::Low)
    );

    let mut uart = Uart::new_blocking(p.UART0, p.PIN_0, p.PIN_1, Default::default());
    let mut rs485_pin = Output::new(p.PIN_2, Level::Low);

    let configuration = loop {
        defmt::info!("Waiting for initialization packet.");
        let packet = receive(&mut uart);
        if let cmri::packet::Payload::Initialization { node_sort: cmri::NodeSort::Cpmega { configuration } } = packet.payload() {
            if configuration.input_bytes() == 1 && configuration.output_bytes() == 1 {
                defmt::debug!("Received configuration: {}", defmt::Debug2Format(configuration));
                break *configuration;
            } else {
                defmt::error!("Received initialization packet with incorrect inputs/outputs: {}", defmt::Debug2Format(configuration));
            }
        }
    };

    defmt::info!("Starting node (address {}).", NODE_ADDRESS);
    let node_address = cmri::Address::try_from_node_address(NODE_ADDRESS).unwrap();
    let mut state: u8 = 0;
    let mut last_data = 0;
    loop {
        match receive(&mut uart).payload() {
            cmri::packet::Payload::Initialization { node_sort } => {
                defmt::debug!("Received initialization packet: {}", defmt::Debug2Format(&node_sort));
                if !matches!(node_sort, cmri::NodeSort::Cpmega { configuration }) {
                    panic!("Received a different configuration");
                }
            },
            cmri::packet::Payload::ReceiveData { .. } => defmt::error!("Node with conflicting address exists!"),
            cmri::packet::Payload::TransmitData { data } => {
                defmt::debug!("Received data packet: {}", data.as_slice());
                state = data[0] & 0b1111_0000;
                if data[0] & 0b0000_0001 != 0 { outputs.0.set_high(); } else { outputs.0.set_low(); }
                if data[0] & 0b0000_0010 != 0 { outputs.1.set_high(); } else { outputs.1.set_low(); }
                if data[0] & 0b0000_0100 != 0 { outputs.2.set_high(); } else { outputs.2.set_low(); }
                if data[0] & 0b0000_1000 != 0 { outputs.3.set_high(); } else { outputs.3.set_low(); }
            },
            cmri::packet::Payload::PollRequest => {
                defmt::debug!("Received poll request");
                let mut data = state;
                if inputs.0.is_high() { data |= 0b0000_0001; }
                if inputs.1.is_high() { data |= 0b0000_0010; }
                if inputs.2.is_high() { data |= 0b0000_0100; }
                if inputs.3.is_high() { data |= 0b0000_1000; }
                let packet = if configuration.options().contains(cmri::node_configuration::CpmegaOptions::CAN_SEND_EOT_ON_NO_INPUTS_CHANGED) && last_data == data {
                    cmri::packet::Packet::new_receive_data(node_address, [].try_into().unwrap())
                } else {
                    cmri::packet::Packet::new_receive_data(node_address, [data].try_into().unwrap())
                };
                let delay = configuration.transmit_delay() as u64;
                if delay > 0 {
                    defmt::trace!("Delaying for {}0µs", delay);
                    embassy_time::Timer::after_micros(delay * 10).await;
                }
                match transmit(&mut uart, &mut rs485_pin, &packet) {
                    Err(error) => defmt::panic!("Error writing CMRInet: {}", defmt::Debug2Format(&error)),
                    Ok(()) => defmt::debug!("Sent data packet: {}", data)
                }
                last_data = data;
            }
        }
    }
}

fn receive(uart: &mut Uart<embassy_rp::peripherals::UART0, embassy_rp::uart::Blocking>) -> cmri::packet::Packet {
    defmt::trace!("Receiving packet");
    use cmri::frame::Raw;

    let mut raw = Raw::new();
    let mut buffer = [0];

    loop {
        match uart.blocking_read(&mut buffer) {
            Err(error) => {
                defmt::error!("Error reading CMRInet: {}", defmt::Debug2Format(&error));
                raw.reset();
            },
            Ok(()) => {
                match raw.receive(buffer[0]) {
                    Err(error) => defmt::panic!("Error reading CMRInet: {}", defmt::Debug2Format(&error)),
                    Ok(complete) => {
                        if complete {
                            if raw.address().is_some_and(|a| a == NODE_ADDRESS) {
                                match raw.try_as_packet() {
                                    Err(error) => defmt::warn!("Received bad packet: {}", defmt::Debug2Format(&error)),
                                    Ok(packet) => return packet
                                }
                            } else {
                                defmt::debug!("Ignoring frame for {}", raw.address());
                                raw.reset();
                            }
                        }
                    }
                }
            }
        }
    }
}

fn transmit(uart: &mut Uart<embassy_rp::peripherals::UART0, embassy_rp::uart::Blocking>, rs485_pin: &mut Output<impl embassy_rp::gpio::Pin>, packet: &cmri::packet::Packet) -> Result<(), embassy_rp::uart::Error> {
    defmt::trace!("Sending packet: {}", defmt::Debug2Format(&packet));
    let frame = packet.encode_frame();
    defmt::trace!("Sending frame: {}", frame.as_slice());
    rs485_pin.set_high();
    let result = uart.blocking_write(frame.as_slice());
    while uart.busy() { defmt::debug!("Flushing"); uart.blocking_flush(); }
    defmt::assert!(!uart.busy());
    rs485_pin.set_low();
    result
}
