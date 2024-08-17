pub fn command() -> clap::Command {
    clap::Command::new("node")
        .bin_name("node")
        .version(clap::crate_version!())
        .about("Simulate a nodes on a CMRInet network - see its outputs and set its inputs")
        .next_line_help(true)
        .group(
            clap::ArgGroup::new("connection")
                .args(["serial", "network"])
                .requires("node-address")
        )
        .arg(common::serial())
        .arg(common::network())

        .arg(common::load_nodes())
        .arg(
            clap::Arg::new("node-address")
                .long("node-address")
                .value_name("<ADDRESS>")
                .value_parser(clap::value_parser!(u8).range(0..=127))
                .help("Address for the node")
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
