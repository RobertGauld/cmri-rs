pub fn command() -> clap::Command {
    clap::Command::new("nodes")
        .bin_name("nodes")
        .version(clap::crate_version!())
        .about("Simulate the nodes on a CMRInet network - see their outputs and set their inputs")
        .next_line_help(true)
        .group(
            clap::ArgGroup::new("connection")
                .args(["serial", "network"])
        )
        .arg(common::serial())
        .arg(common::network())

        .arg(
            clap::Arg::new("open-node")
                .long("open-node")
                .value_name("ADDRESS")
                .value_parser(clap::value_parser!(u8).range(0..=127))
                .required(false)
                .help("Open the node window for a given node when connected")
                .value_parser(clap::value_parser!(u8).range(..=127))
        )

        .arg(common::load_nodes())
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
