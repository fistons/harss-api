use std::env::var;

use opentelemetry::sdk::trace::{self, RandomIdGenerator, Sampler, Tracer};
use tracing::{subscriber::set_global_default, Subscriber};
use tracing_log::LogTracer;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

pub fn get_subscriber(name: &str, env_filter: &str) -> impl Subscriber + Sync + Send {
    let telemetry = build_datadog(name).or_else(|| build_jaeger(name));

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let fmt = tracing_subscriber::fmt::Layer::new();

    Registry::default()
        .with(telemetry)
        .with(env_filter)
        .with(fmt)
}

pub fn init_subscriber(subscriber: impl Subscriber + Sync + Send) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}

fn build_jaeger<S>(name: &str) -> Option<OpenTelemetryLayer<S, Tracer>>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    // Building the jaeger layer, if needed
    if var("JAEGER_ENABLED").is_ok() {
        opentelemetry_jaeger::new_agent_pipeline()
            .with_service_name(name)
            .install_batch(opentelemetry::runtime::Tokio)
            .map_err(|err| eprintln!("Datadog error {:?}", err))
            .ok()
            .map(|x| tracing_opentelemetry::layer().with_tracer(x))
    } else {
        None
    }
}

fn build_datadog<S>(name: &str) -> Option<OpenTelemetryLayer<S, Tracer>>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    if var("DD_ENABLED").is_ok() {
        opentelemetry_datadog::new_pipeline()
            .with_service_name(name)
            .with_agent_endpoint(
                var("DD_AGENT").unwrap_or_else(|_| "http://127.0.0.1:8126".to_owned()),
            )
            .with_trace_config(
                trace::config()
                    .with_sampler(Sampler::AlwaysOn) //TODO Add sampling, one day.
                    .with_id_generator(RandomIdGenerator::default()),
            )
            .install_batch(opentelemetry::runtime::Tokio)
            .map_err(|err| eprintln!("Datadog error {:?}", err))
            .ok()
            .map(|x| tracing_opentelemetry::layer().with_tracer(x))
    } else {
        None
    }
}
