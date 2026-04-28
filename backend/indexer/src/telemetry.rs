use opentelemetry::global;
use opentelemetry::propagation::Injector;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::Resource;

pub fn init_tracing(service_name: &str) {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "indexer=info".into());
    let fmt_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stdout);

    global::set_text_map_propagator(TraceContextPropagator::new());

    let otlp_endpoint = std::env::var("OTLP_ENDPOINT")
        .or_else(|_| std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT"))
        .ok();
    let service_name =
        std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| service_name.to_string());

    if let Some(endpoint) = otlp_endpoint {
        let trace_config =
            opentelemetry_sdk::trace::Config::default().with_resource(Resource::new(vec![
                KeyValue::new("service.name", service_name),
            ]));

        match opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_trace_config(trace_config)
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(endpoint),
            )
            .install_batch(opentelemetry_sdk::runtime::Tokio)
        {
            Ok(tracer) => {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt_layer)
                    .with(tracing_opentelemetry::layer().with_tracer(tracer))
                    .init();
                return;
            }
            Err(err) => {
                eprintln!("Failed to initialize OTLP tracing: {err}");
            }
        }
    }

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}

pub fn inject_current_trace_context(headers: &mut reqwest::header::HeaderMap) {
    let context = opentelemetry::Context::current();
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&context, &mut ReqwestHeaderInjector(headers));
    });
}

struct ReqwestHeaderInjector<'a>(&'a mut reqwest::header::HeaderMap);

impl Injector for ReqwestHeaderInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        if let (Ok(header_name), Ok(header_value)) = (
            reqwest::header::HeaderName::from_bytes(key.as_bytes()),
            reqwest::header::HeaderValue::from_str(&value),
        ) {
            self.0.insert(header_name, header_value);
        }
    }
}
