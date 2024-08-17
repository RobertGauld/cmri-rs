pub fn command() -> clap::Command {
    clap::Command::new("hub")
        .bin_name("hub")
        .version(clap::crate_version!())
        .about("Links multiple CMRInet networks")
        .long_about("A message received on any connection will be sent out all the others. Logs info messages to STDOUT when connections are made/lost, logs debug messages to STDOUT when a packet is received")
        .next_line_help(true)
        .group(
            clap::ArgGroup::new("connection")
                .args(["serial", "network", "server"])
                .multiple(true)
        )
        .arg(common::serial().action(clap::ArgAction::Append))
        .arg(common::network().action(clap::ArgAction::Append))
        .arg(
            clap::Arg::new("server")
                .long("server")
                .value_name("ADDRESS:PORT")
                .value_hint(clap::ValueHint::Hostname)
                .help("Start a TCP server and wait for connections on ADDRESS:PORT (e.g. \"127.0.0.1:7878\")")
        )
        .arg(
            clap::Arg::new("gui")
                .long("no-gui")
                .help("Don't show the graphical user interface")
                .action(clap::ArgAction::SetFalse)
        )
}

mod common {
    include!("../../cli/args.rs");
}

#[cfg(test)]
mod tests {
    #[test]
    fn verify_command() {
        super::command().debug_assert();
    }
}
