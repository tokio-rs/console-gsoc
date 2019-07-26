use tracing_core::field::Visit;
use tracing_core::span;

use std::fmt::Debug;

include!(concat!(env!("OUT_DIR"), "/tracing.rs"));

#[derive(Default)]
pub struct Recorder(pub Vec<Value>);

impl Visit for Recorder {
    fn record_debug(&mut self, field: &tracing_core::Field, value: &dyn Debug) {
        self.0.push(Value {
            field: Some(Field {
                name: field.name().to_string(),
            }),
            value: Some(value::Value::Debug(DebugRecord {
                debug: format!("{:?}", value),
                pretty: format!("{:#?}", value),
            })),
        })
    }

    fn record_i64(&mut self, field: &tracing_core::Field, value: i64) {
        self.0.push(Value {
            field: Some(Field {
                name: field.name().to_string(),
            }),
            value: Some(value::Value::Signed(value)),
        })
    }
    fn record_u64(&mut self, field: &tracing_core::Field, value: u64) {
        self.0.push(Value {
            field: Some(Field {
                name: field.name().to_string(),
            }),
            value: Some(value::Value::Unsigned(value)),
        })
    }
    fn record_bool(&mut self, field: &tracing_core::Field, value: bool) {
        self.0.push(Value {
            field: Some(Field {
                name: field.name().to_string(),
            }),
            value: Some(value::Value::Boolean(value)),
        })
    }
    fn record_str(&mut self, field: &tracing_core::Field, value: &str) {
        self.0.push(Value {
            field: Some(Field {
                name: field.name().to_string(),
            }),
            value: Some(value::Value::Str(value.to_string())),
        })
    }
}

impl From<&span::Id> for SpanId {
    fn from(id: &span::Id) -> Self {
        SpanId { id: id.into_u64() }
    }
}

impl From<&'static tracing_core::Metadata<'static>> for Metadata {
    fn from(meta: &tracing_core::Metadata) -> Self {
        let fieldset = meta
            .fields()
            .iter()
            .map(|field| Field {
                name: field.name().to_string(),
            })
            .collect();

        let level = match *meta.level() {
            tracing_core::Level::DEBUG => Level::Debug,
            tracing_core::Level::ERROR => Level::Error,
            tracing_core::Level::INFO => Level::Info,
            tracing_core::Level::TRACE => Level::Trace,
            tracing_core::Level::WARN => Level::Warn,
        }
        .into();

        Metadata {
            fieldset,
            level,
            name: meta.name().to_string(),
            target: meta.name().to_string(),
            module_path: meta.name().to_string(),
            file: meta.name().to_string(),
            line: meta.line().map(|num| LineNum { num }),
            is_event: meta.is_event(),
            is_span: meta.is_span(),
        }
    }
}

impl<'a> From<&'a span::Attributes<'a>> for Attributes {
    fn from(attr: &span::Attributes) -> Self {
        Attributes {
            metadata: Some(attr.metadata().into()),
            is_root: attr.is_root(),
            is_contextual: attr.is_contextual(),
            parent: attr.parent().map(|id| SpanId { id: id.into_u64() }),
        }
    }
}
