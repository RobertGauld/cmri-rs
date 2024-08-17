//! A simple GUI app for "simulating" the nodes of a CMRInet.
//! The user can view the outputs set by the controller and set the inputs.

mod cli;
mod gui;
mod state;

#[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
fn main() -> anyhow::Result<()> {
    cmri_tools::init_tracing(
        tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("nodes=info".parse()?)
    );

    let runtime = cmri_tools::tokio_runtime(2)?;
    let cli_args = cli::command().get_matches();

    gui::run(&cli_args, runtime.handle().clone());
    Ok(())
}
