use anyhow::Result;
use tracing::{subscriber::set_global_default, Subscriber};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, EnvFilter, Registry};

// Setup a default telemetry subscriber that prints to the console
fn get_subscriber(env_filter: &str) -> impl Subscriber + Sync + Send {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let sub = Registry::default().with(env_filter);
    sub.with(Layer::default().with_level(false).with_target(false))
}

// Initialize a standard simple telemetry package:
// - Forward all logs as tracing events
// - Collect all tracing events and print those matching env_filter or higher to the console
// Value env_filter levels (in decreasing levels): Error, Warn, Info, Debug, Trace
pub fn init_telemetry(env_filter: &str) -> Result<()> {
    LogTracer::init()?;
    set_global_default(get_subscriber(env_filter))?;
    Ok(())
}
