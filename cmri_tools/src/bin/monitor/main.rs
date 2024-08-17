//! A simple GUI app for monitoring a CMRInet bus.

mod cli;
mod gui;
mod monitor;

#[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
fn main() -> anyhow::Result<()> {
    cmri_tools::init_tracing(
        tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("monitor=info".parse()?)
    );

    let runtime = cmri_tools::tokio_runtime(2)?;
    let cli_args = cli::command().get_matches();

    gui::run(&cli_args, runtime.handle().clone());
    Ok(())
}
