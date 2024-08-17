#![no_std]
#![no_main]

//! Simulates a CMRInet network with a controller and several nodes.
//! Useful for testing the monitor app.
//! Debugging information is handled by defmt and probe-rs.
//! UART0 (gpio0 TX, gpio1 RX, 115200 baud, 8 data bits, 1 stop bit, no parity)
//! is used for the CMRInet network, gpio2 (high for TX) is available for controlling an RS485 module.

use embassy_executor::Spawner;
use embassy_rp::adc::{Adc, Channel, Config};
use embassy_time::Instant;
use defmt::{info, debug};
use {defmt_rtt as _, panic_probe as _};
use rand::{rngs::SmallRng, SeedableRng};

mod cmri_bus;
mod node;
use node::Node;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("CMRInet Simulator.");
    unsafe { embassy_rp::time_driver::init(); }
    let p = embassy_rp::init(Default::default());
    let mut bus = cmri_bus::new(p.UART0, p.PIN_0, p.PIN_1, Default::default(), p.PIN_2);
    let mut adc = embassy_rp::adc::Adc::new_blocking(p.ADC, Config::default());
    let mut p28 = Channel::new_pin(p.PIN_28, embassy_rp::gpio::Pull::None);
    let mut random = create_prng(&mut adc, &mut p28);

    let started = Instant::now();
    let mut nodes = [
        Node::new_cpmega(&mut random, 10, 0, cmri::node_configuration::CpmegaOptions::default(), 16, 8),
        Node::new_cpnode(&mut random, 11, 0, cmri::node_configuration::CpnodeOptions::default(), 4, 4),
        Node::new_smini(&mut random, 12, 0),
        Node::new_usic(&mut random, 13, 0, 16, 32),
        Node::new_susic(&mut random, 14, 0, 128, 128),
    ];
    let finished = Instant::now();
    debug!("Created nodes in {}µs.", (finished - started).as_micros());

    info!("Sending initialization packets.");
    for node in nodes.iter() {
        node.transmit_initialization(&mut bus);
    }

    let period = embassy_time::Duration::from_millis(50);
    info!("Simulating {} nodes (looping every {}ms).", nodes.len(), period.as_millis());
    let mut ticker = embassy_time::Ticker::every(period);
    loop {
        let started = Instant::now();

        for node in &mut nodes {
            node.transmit_poll_receive(&mut bus);
        }

        for node in &mut nodes {
            node.transmit_transmit(&mut bus);
        }

        random = create_prng(&mut adc, &mut p28);
        for node in &mut nodes {
            node.shuffle_data(&mut random);
        }
        let finished = Instant::now();
        debug!("Loop took {}µs", (finished - started).as_micros());
        ticker.next().await;
    }
}

/// Create a pseudo-random number generator, seeded by reading from adc.
#[expect(clippy::cast_possible_truncation)]
fn create_prng(adc: &mut Adc<embassy_rp::adc::Blocking>, channel: &mut Channel) -> SmallRng {
    let started = Instant::now();
    let seed = u64::from_ne_bytes([
        adc.blocking_read(channel).unwrap() as u8,
        adc.blocking_read(channel).unwrap() as u8,
        adc.blocking_read(channel).unwrap() as u8,
        adc.blocking_read(channel).unwrap() as u8,
        adc.blocking_read(channel).unwrap() as u8,
        adc.blocking_read(channel).unwrap() as u8,
        adc.blocking_read(channel).unwrap() as u8,
        adc.blocking_read(channel).unwrap() as u8
    ]);
    let random = SmallRng::seed_from_u64(seed);
    let finished = Instant::now();
    debug!("Created PRNG in {}µs", (finished - started).as_micros());
    random
}
