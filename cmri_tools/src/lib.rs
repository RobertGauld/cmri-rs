#![doc = include_str!("../README.md")]

pub mod connection;
pub mod file;
pub mod readings;

pub mod gui;

use std::time::Duration;

/// Initialze tracing with the provided `tracing_subscriber::EnvFilter`.
///
/// # Example
///
/// ```
/// cmri_tools::init_tracing(
///     tracing_subscriber::EnvFilter::from_default_env()
///         .add_directive("bin_name=info".parse().unwrap())
/// );
/// ```
pub fn init_tracing(filter: tracing_subscriber::EnvFilter) {
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .init();
}

/// Get a `tokio` `Runtime` configured with time and io.
#[expect(clippy::missing_errors_doc)]
pub fn tokio_runtime(threads: usize) -> std::io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(threads)
        .enable_io()
        .enable_time()
        .build()
}

#[inline]
/// Truncate the precision of a `Duration` to milliseconds for easier human readability.
///
/// # Panics
///
/// In the unlikly event that the number of milliseconds overflows a `u64`.
#[must_use]
pub fn truncate_duration_to_millis(duration: &Duration) -> Duration {
    Duration::from_millis(
        duration.as_millis().try_into().expect("Value not to exceed 584,542,000 years")
    )
}

#[inline]
/// Truncate the precision of a `Duration` to microseconds for easier human readability.
///
/// # Panics
///
/// In the unlikly event that the number of microseconds overflows a `u64`.
#[must_use]
pub fn truncate_duration_to_micros(duration: &Duration) -> Duration {
    Duration::from_micros(
        duration.as_micros().try_into().expect("Value not to exceed 584,542 years")
    )
}


#[allow(clippy::missing_panics_doc, reason="tests")]
#[cfg(test)]
mod tests {
    use std::time::Duration;

    #[test]
    fn truncate_duration_to_millis() {
        assert_eq!(
            super::truncate_duration_to_millis(&Duration::from_micros(1_001)),
            Duration::from_millis(1)
        );

        assert_eq!(
            super::truncate_duration_to_millis(&Duration::from_micros(1_001)),
            Duration::from_micros(1000)
        );
    }

    #[test]
    fn truncate_duration_to_micros() {
        assert_eq!(
            super::truncate_duration_to_micros(&Duration::from_nanos(1_001)),
            Duration::from_micros(1)
        );

        assert_eq!(
            super::truncate_duration_to_micros(&Duration::from_nanos(1_001)),
            Duration::from_nanos(1000)
        );
    }
}
