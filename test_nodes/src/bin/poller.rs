#![no_std]
#![no_main]

//! Runs a simple CMRInet controller on a Raspberry Pi Pico.
//! Polls a CPMega with 1 input byte and 1 output byte.
//! Pins gpio6 (low bit) to gpio13 (high bit) follow the input byte.
//! Pins gpio14 (low bit) to gpio21 (high bit) control the output byte.
//! Debugging information is handled by defmt and probe-rs.
//! UART0 (gpio0 TX, gpio1 RX, 115200 baud, 8 data bits, 1 stop bit, no parity)
//! is used for the CMRInet network, gpio2 (high for TX) is available for controlling an RS485 module.
const NODE_ADDRESS: u8 = 5; // 0 - 127 (inclusive)
const TIMEOUT: embassy_time::Duration = embassy_time::Duration::from_millis(100);
const PERIOD: embassy_time::Duration = embassy_time::Duration::from_millis(250);

use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Output, Level, Pull};
use embassy_rp::uart::Uart;
use embassy_time::Instant;
use defmt::{error, warn, info, debug, trace, panic, Debug2Format};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    unsafe { embassy_rp::time_driver::init(); }
    let p = embassy_rp::init(Default::default());

    let inputs = (
        Input::new(p.PIN_14, Pull::Down),
        Input::new(p.PIN_15, Pull::Down),
        Input::new(p.PIN_16, Pull::Down),
        Input::new(p.PIN_17, Pull::Down),
        Input::new(p.PIN_18, Pull::Down),
        Input::new(p.PIN_19, Pull::Down),
        Input::new(p.PIN_20, Pull::Down),
        Input::new(p.PIN_21, Pull::Down)
    );

    let mut outputs = (
        Output::new(p.PIN_6, Level::Low),
        Output::new(p.PIN_7, Level::Low),
        Output::new(p.PIN_8, Level::Low),
        Output::new(p.PIN_9, Level::Low),
        Output::new(p.PIN_10, Level::Low),
        Output::new(p.PIN_11, Level::Low),
        Output::new(p.PIN_12, Level::Low),
        Output::new(p.PIN_13, Level::Low)
    );

    embassy_rp::bind_interrupts!(struct UartInt {
        UART0_IRQ => embassy_rp::uart::InterruptHandler<embassy_rp::peripherals::UART0>;
    });
    let mut uart = Uart::new(p.UART0, p.PIN_0, p.PIN_1, UartInt, p.DMA_CH10, p.DMA_CH11, Default::default());
    let mut rs485_pin = Output::new(p.PIN_2, Level::Low);

    let node_address = cmri::Address::try_from_node_address(NODE_ADDRESS).unwrap();
    let mut to_initialize = true;
    let mut ticker = embassy_time::Ticker::every(PERIOD);
    let configuration = cmri::node_configuration::CpmegaConfiguration::try_new(0, cmri::node_configuration::CpmegaOptions::default(), 1, 1).unwrap();
    let initialization_packet = cmri::packet::Packet::new_initialization(node_address, cmri::NodeSort::Cpmega { configuration });
    let poll_packet = cmri::packet::Packet::new_poll_request(node_address);

    info!("Polling node {}", NODE_ADDRESS);
    loop {
        if to_initialize {
            info!("Sending initialization packet.");
            match transmit(&mut uart, &mut rs485_pin, &initialization_packet).await {
                Err(error) => panic!("Error writing CMRInet: {}", Debug2Format(&error)),
                Ok(()) => {
                    debug!("Sent initialization packet: {}", Debug2Format(&initialization_packet));
                    to_initialize = false;
                }
            }
        }


        info!("Polling");
        match transmit(&mut uart, &mut rs485_pin, &poll_packet).await {
            Err(error) => panic!("Error writing CMRInet: {}", Debug2Format(&error)),
            Ok(()) => {
                let polled_at = Instant::now();
                debug!("Sent poll packet: {}", Debug2Format(&poll_packet));

                let receive_future = async {
                    loop {
                        let packet = receive(&mut uart).await;
                        if let cmri::packet::Payload::ReceiveData { data } = packet.payload() {
                            let received_at = Instant::now();
                            debug!("Received data: {}", Debug2Format(&packet));
                            break (data[0], received_at)
                        }
                    }
                };
                match embassy_time::with_timeout(TIMEOUT, receive_future).await {
                    Err(embassy_time::TimeoutError) => {
                        error!("Timed out waiting for response from node.");
                        to_initialize = true;
                    },
                    Ok((data, received_at)) => {
                        info!("Inputs: {} {:02x} {:08b}", data, data, data);
                        info!("Node responded in {}µs", (received_at - polled_at).as_micros());
                        if data & 0b0000_0001 != 0 { outputs.0.set_high(); } else { outputs.0.set_low(); }
                        if data & 0b0000_0010 != 0 { outputs.1.set_high(); } else { outputs.1.set_low(); }
                        if data & 0b0000_0100 != 0 { outputs.2.set_high(); } else { outputs.2.set_low(); }
                        if data & 0b0000_1000 != 0 { outputs.3.set_high(); } else { outputs.3.set_low(); }
                        if data & 0b0001_0000 != 0 { outputs.4.set_high(); } else { outputs.4.set_low(); }
                        if data & 0b0010_0000 != 0 { outputs.5.set_high(); } else { outputs.5.set_low(); }
                        if data & 0b0100_0000 != 0 { outputs.6.set_high(); } else { outputs.6.set_low(); }
                        if data & 0b1000_0000 != 0 { outputs.7.set_high(); } else { outputs.7.set_low(); }
                    }
                }
            }
        }

        embassy_time::Timer::after_millis(50).await;
        let mut data: u8 = 0b0000_0000;
        if inputs.0.is_high() { data |= 0b0000_0001; }
        if inputs.1.is_high() { data |= 0b0000_0010; }
        if inputs.2.is_high() { data |= 0b0000_0100; }
        if inputs.3.is_high() { data |= 0b0000_1000; }
        if inputs.4.is_high() { data |= 0b0001_0000; }
        if inputs.5.is_high() { data |= 0b0010_0000; }
        if inputs.6.is_high() { data |= 0b0100_0000; }
        if inputs.7.is_high() { data |= 0b1000_0000; }
        info!("Transmitting data {} {:02x} {:08b}", data, data, data);
        let packet = cmri::packet::Packet::new_transmit_data(node_address, [data].try_into().unwrap());
        match transmit(&mut uart, &mut rs485_pin, &packet).await {
            Err(error) => panic!("Error writing CMRInet: {}", Debug2Format(&error)),
            Ok(()) => debug!("Sent data packet: {}", Debug2Format(&packet))
        }
        ticker.next().await;
    }
}

async fn receive(uart: &mut Uart<'_, impl embassy_rp::uart::Instance, embassy_rp::uart::Async>) -> cmri::packet::Packet {
    trace!("Receiving packet");
    use cmri::frame::Raw;

    let mut raw = Raw::new();
    let mut buffer = [0];

    let instant1 = Instant::now();
    let mut instant2 = None;
    loop {
        let read = uart.read(&mut buffer).await;
        if instant2.is_none() { instant2 = Some(Instant::now()) }
        match read {
            Err(error) => panic!("Error reading CMRInet: {}", Debug2Format(&error)),
            Ok(()) => {
                match raw.receive(buffer[0]) {
                    Err(error) => panic!("Error reading CMRInet: {}", Debug2Format(&error)),
                    Ok(complete) => {
                        if complete {
                            if raw.address().is_some_and(|a| a == NODE_ADDRESS) {
                                let instant3 = Instant::now();
                                match raw.try_as_packet() {
                                    Err(error) => warn!("Received bad packet: {}", Debug2Format(&error)),
                                    Ok(packet) => {
                                        if let Some(instant2) = instant2 {
                                            let instant4 = Instant::now();
                                            debug!("Waiting: {}µs, Receiving: {}µs, Parsing: {}µs", (instant2 - instant1).as_micros(), (instant3 - instant2).as_micros(), (instant4 - instant3).as_micros());
                                        }
                                        return packet;
                                    }
                                }
                            } else {
                                debug!("Ignoring frame for {}", raw.address());
                                instant2 = None;
                                raw.clear();
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn transmit(uart: &mut Uart<'_, impl embassy_rp::uart::Instance, embassy_rp::uart::Async>, rs485_pin: &mut Output<'_, impl embassy_rp::gpio::Pin>, packet: &cmri::packet::Packet) -> Result<(), embassy_rp::uart::Error> {
    let instant1 = Instant::now();
    debug!("Sending packet: {}", Debug2Format(&packet));
    let frame = packet.encode_frame();
    trace!("Sending frame: {}", frame.as_slice());
    let instant2 = Instant::now();
    rs485_pin.set_high();
    let result = uart.write(frame.as_slice()).await;
    while uart.busy() { embassy_futures::yield_now().await; }
    rs485_pin.set_low();
    let instant3 = Instant::now();
    debug!("Making frame: {}µs, Transmitting frame: {}µs", (instant2 - instant1).as_micros(), (instant3 - instant2).as_micros());
    result
}
