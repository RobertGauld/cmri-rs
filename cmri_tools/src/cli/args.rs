#[allow(dead_code, reason = "Included in several files.")]
pub fn serial() -> clap::Arg {
    clap::Arg::new("serial")
        .long("serial")
        .value_name("PORT[:BAUD]")
        .required(false)
        .value_hint(clap::ValueHint::AnyPath)
        .help("Serial port to use (e.g. \"/dev/ttyACM0\", \"/dev/ttyACM1:9600\", \"COM4:115200\")")
}

#[allow(dead_code, reason = "Included in several files.")]
pub fn network() -> clap::Arg {
    clap::Arg::new("network")
        .long("network")
        .value_name("ADDRESS:PORT")
        .required(false)
        .value_hint(clap::ValueHint::Hostname)
        .help("Connect to a TCP server at ADDRESS:PORT (e.g. \"127.0.0.1:7878\")")
}

#[allow(dead_code, reason = "Included in several files.")]
pub fn load_nodes() -> clap::Arg {
    clap::Arg::new("load-nodes")
        .long("load-nodes")
        .value_name("FILE")
        .required(false)
        .help("Load initial nodes from a file")
        .value_parser(clap::value_parser!(std::path::PathBuf))
}
