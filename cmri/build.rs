//! * Sets toolchain cfg value, so that benchmarks can be conditionally compiled.
//!   (To be removed after <https://doc.rust-lang.org/unstable-book/library-features/test.html> is in stable).

use std::process::Command;

#[allow(clippy::unwrap_used)]
fn main() {
    let mut command = Command::new("rustc");
    command.arg("--version");
    let output = String::from_utf8(command.output().unwrap().stdout).unwrap();
    let output = output.trim();

    let parts: Vec<&str> = output.split(' ').skip(1).collect();
    let (_version, toolchain) = parts[0].split_once('-').unwrap_or((parts[0], "stable"));

    println!("cargo::rustc-check-cfg=cfg(toolchain, values(\"stable\", \"nightly\"))");
    println!("cargo:rustc-cfg=toolchain={toolchain:?}");
}
