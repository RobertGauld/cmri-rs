//! * Generate shell autocompletion files.

use clap::ValueEnum;
use clap_complete::{generate_to, Shell};
use std::env;
use std::io::Error;

mod hub { include!("src/bin/hub/cli.rs"); }
mod monitor { include!("src/bin/monitor/cli.rs"); }
mod controller { include!("src/bin/controller/cli.rs"); }
mod nodes { include!("src/bin/nodes/cli.rs"); }
mod node { include!("src/bin/node/cli.rs"); }

fn main() -> Result<(), Error> {
    let out_dir = {
        let out_dir = env::var_os("OUT_DIR").expect("ENV[OUT_DIR] to have a value.");
        let mut out_dir = std::path::PathBuf::from(out_dir);
        out_dir.push("autocomplete");
        if !out_dir.as_path().exists() {
            std::fs::create_dir(&out_dir)?;
        }
        out_dir
    };

    let mut commands = [hub::command(), monitor::command(), controller::command(), nodes::command(), node::command()];

    for &gen in Shell::value_variants() {
        let mut out_dir = out_dir.clone();
        out_dir.push(gen.to_string());
        if !out_dir.as_path().exists() {
            std::fs::create_dir(&out_dir)?;
        }
        for cmd in &mut commands {
            let bin_name = cmd.get_bin_name().expect("Expected command to have bin_name.").to_string();
            let _ = generate_to(gen, cmd, bin_name.clone(), &out_dir)?;
        }
    }

    Ok(())
}
