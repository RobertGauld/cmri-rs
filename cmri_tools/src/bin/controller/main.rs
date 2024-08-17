//! A simple GUI app for controlling the nodes of a CMRInet.
//! The user can view the inputs and set the outputs of each node.

mod cli;
mod gui;
mod controller;

#[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
fn main() -> anyhow::Result<()> {
    cmri_tools::init_tracing(
        tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("controller=info".parse()?)
    );

    let runtime = cmri_tools::tokio_runtime(2)?;
    let cli_args = cli::command().get_matches();

    gui::run(&cli_args, runtime.handle().clone());
    Ok(())
}
