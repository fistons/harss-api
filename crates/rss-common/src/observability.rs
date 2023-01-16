use opentelemetry::sdk::trace::Tracer;
use sentry::ClientInitGuard;
use tracing::{subscriber::set_global_default, Subscriber};
use tracing_log::LogTracer;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

pub fn get_subscriber(name: &str, env_filter: &str) -> impl Subscriber + Sync + Send {
    // Building the jaeger layer, if needed
    let jaeger = build_jaeger(name);

    let datadog = opentelemetry_datadog::new_pipeline()
        .with_service_name(name)
        .install_batch(opentelemetry::runtime::Tokio)
        .ok()
        .map(|x| tracing_opentelemetry::layer().with_tracer(x));

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let fmt = tracing_subscriber::fmt::Layer::new();

    Registry::default()
        .with(env_filter)
        .with(fmt)
        .with(sentry_tracing::layer())
        .with(jaeger)
        .with(datadog)
}
pub fn init_subscriber(subscriber: impl Subscriber + Sync + Send) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}

pub fn init_sentry() -> ClientInitGuard {
    sentry::init(sentry::ClientOptions {
        release: sentry::release_name!(),
        ..Default::default()
    })
}

fn build_jaeger<S>(name: &str) -> Option<OpenTelemetryLayer<S, Tracer>>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    // Building the jaeger layer, if needed
    if std::env::var("JAEGER_ENABLED").is_ok() {
        let tracer = opentelemetry_jaeger::new_agent_pipeline()
            .with_service_name(name)
            .install_batch(opentelemetry::runtime::Tokio)
            .unwrap();
        Some(tracing_opentelemetry::layer().with_tracer(tracer))
    } else {
        None
    }
}
