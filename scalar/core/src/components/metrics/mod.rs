pub mod registry;

use std::collections::HashMap;

pub use prometheus::core::Collector;
pub use prometheus::{
    labels, Counter, CounterVec, Error as PrometheusError, Gauge, GaugeVec, Histogram,
    HistogramOpts, HistogramVec, Opts, Registry,
};
pub use registry::MetricsRegistry;

/// Create an unregistered counter with labels
pub fn counter_with_labels(
    name: &str,
    help: &str,
    const_labels: HashMap<String, String>,
) -> Result<Counter, PrometheusError> {
    let opts = Opts::new(name, help).const_labels(const_labels);
    Counter::with_opts(opts)
}

/// Create an unregistered gauge with labels
pub fn gauge_with_labels(
    name: &str,
    help: &str,
    const_labels: HashMap<String, String>,
) -> Result<Gauge, PrometheusError> {
    let opts = Opts::new(name, help).const_labels(const_labels);
    Gauge::with_opts(opts)
}
