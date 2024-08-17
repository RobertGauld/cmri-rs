//! A CLI/GUI application which provids the means to interconnect devices/software
//! which have differing connection requirements, anything received on a connection
//! is written to all the others.

use anyhow::Context;
use tracing::info;
use tokio::sync::Mutex;
use std::sync::Arc;

mod cli;
mod gui;
mod hub;
use hub::{Hub, state::State};

#[allow(clippy::missing_panics_doc, clippy::missing_errors_doc)]
fn main() -> anyhow::Result<()> {
    cmri_tools::init_tracing(
        tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("hub=info".parse()?)
    );

    let runtime = cmri_tools::tokio_runtime(4)?;
    let cli = cli::command().get_matches();

    let hub_state: anyhow::Result<(Hub, Arc<Mutex<State>>)> = runtime.block_on(async {
        let (hub, state) = hub::new().await;

        // Setup a TCP server
        if let Some(address) = cli.get_one::<String>("server") {
            hub.start_server(address).await.context(format!("Starting TCP server {address:?}."))?;
        }

        // Setup TCP clients
        if let Some(addresses) = cli.get_many::<String>("network") {
            for address in addresses {
                hub.add_network(address).context(format!("Connecting to TCP server {address:?}."))?;
            }
        }

        // Setup Serial ports
        if let Some(addresses) = cli.get_many::<String>("serial") {
            for address in addresses {
                let (port, baud) = cmri_tools::connection::port_baud_from_str(address).context(format!("Parsing serial port {address:?}."))?;
                hub.add_serial_port(port, baud).context(format!("Opening serial port {port:?}."))?;
            }
        }

        Ok((hub, state))
    });
    let (hub, state) = hub_state?;

    // Run the GUI, or print statistics every second.
    if cli.get_flag("gui") {
        gui::run(hub, state, runtime.handle().clone());
    } else {
        loop {
            {
                let state = state.blocking_lock();
                info!(
                    "Frames: {} ({}/s)\nBytes: {} ({}/s)\nConnections:\n",
                    readable::num::Unsigned::from(state.frames().1),
                    readable::num::Unsigned::from(state.frames().2.last().copied().unwrap_or_default()),
                    readable_byte::readable_byte::b(state.bytes().1).to_string_as(true),
                    readable_byte::readable_byte::b(state.bytes().2.last().copied().unwrap_or_default().into()).to_string_as(true)
                );
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
    Ok(())
}
