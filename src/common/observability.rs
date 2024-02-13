use std::env::var;

use tracing::info;
use tracing::{subscriber::set_global_default, Subscriber};
use tracing_log::LogTracer;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

pub fn get_subscriber(name: &str, env_filter: &str) -> impl Subscriber + Sync + Send {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let fmt = tracing_subscriber::fmt::Layer::default();

    Registry::default()
        .with(build_datadog(name))
        .with(build_jaeger(name))
        .with(env_filter)
        .with(fmt)
}

pub fn init_subscriber(subscriber: impl Subscriber + Sync + Send) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}

fn build_jaeger<S>(name: &str) -> Option<OpenTelemetryLayer<S, opentelemetry_sdk::trace::Tracer>>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    var("JAEGER_AGENT_ENDPOINT")
        .map(|jaeger_endpoint| {
            info!("Setting up Jaeger agent on {}", jaeger_endpoint);
            opentelemetry_jaeger::new_agent_pipeline()
                .with_service_name(name)
                .with_endpoint(jaeger_endpoint)
                .install_batch(opentelemetry_sdk::runtime::Tokio)
                .ok()
                .map(|x| tracing_opentelemetry::layer().with_tracer(x))
        })
        .ok()?
}

fn build_datadog<S>(name: &str) -> Option<OpenTelemetryLayer<S, opentelemetry_sdk::trace::Tracer>>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    var("DD_AGENT_ENDPOINT")
        .map(|dd_agent_endpoint| {
            info!("Setting up Datadog agent on {}", dd_agent_endpoint);
            opentelemetry_datadog::new_pipeline()
                .with_service_name(name)
                .with_agent_endpoint(dd_agent_endpoint)
                .install_batch(opentelemetry_sdk::runtime::Tokio)
                .ok()
                .map(|x| tracing_opentelemetry::layer().with_tracer(x))
        })
        .ok()?
}
