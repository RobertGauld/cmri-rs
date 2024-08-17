use embassy_rp::gpio::{Output, Pin, Level};
use embassy_rp::uart::{Uart, Config, Instance, Blocking, TxPin, RxPin};
use embassy_rp::Peripheral;
use defmt::{error, warn, info, debug, trace, panic, Debug2Format};

/// A CMRInet bus running over RS232/RS485.
pub trait Bus {
    fn transmit(&mut self, packet: &cmri::packet::Packet) -> Result<(), embassy_rp::uart::Error>;
}

pub fn new<'d, U: Instance, P: Pin>(
    uart: impl Peripheral<P = U> + 'd,
    tx: impl Peripheral<P = impl TxPin<U>> + 'd,
    rx: impl Peripheral<P = impl RxPin<U>> + 'd,
    config: Config,
    rs485: P
) -> BusImpl<'d, U, P> {
    BusImpl {
        uart: Uart::new_blocking(uart, tx, rx, config),
        rs385_pin: Output::new(rs485, Level::Low)
    }
}

pub struct BusImpl<'d, U: Instance, P: Pin> {
    uart: Uart<'d, U, Blocking>,
    rs385_pin: Output<'d, P>
}
impl<'d, U, P> Bus for BusImpl<'d, U, P> where U: Instance, P: Pin {
    fn transmit(&mut self, packet: &cmri::packet::Packet) -> Result<(), embassy_rp::uart::Error> {
        debug!("Sending packet: {}", Debug2Format(&packet));
        let frame = packet.encode_frame();
        trace!("Sending frame: {}", frame.as_slice());
        self.rs385_pin.set_high();
        let result = self.uart.blocking_write(frame.as_slice());
        while self.uart.busy() {}
        self.rs385_pin.set_low();
        result
    }
}
