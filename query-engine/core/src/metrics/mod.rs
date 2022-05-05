//! Query Engine Metrics
//! This crate is responsible for capturing and recording metrics in the Query Engine.
//! Metrics is broken into two parts, `MetricsRecorder` and `MetricsRegistry`, and uses our tracing framework to communicate.
//! An example best explains this system.
//! When the engine boots up, the `MetricRegistry` is added to our tracing as a layer and The MetricRecorder is
//! set as the global metric recorder. When a metric value is recorded `gauge_increment!("my_gauge", 1.0)` is called.
//! The Metric Recorder is called with `register_gauge` and returns a `MetricHandle`. The `MetricHandle` will convert
//! ``gauge_increment!("my_gauge", 1.0)` into a `trace!` message.
//!
//! The trace message received my the `MetricRegistry` and converted into a `MetricVisitor` with all the information required
//! to record the metric and the value. The MetricVisitor is then processed and the metric value added to the `Registry`.
//!
//! To view the recorded metrics we create a `Snapshot` of our metrics and then return it in `json` format.
//! A few things to note:
//! * We have an `ACCEPT_LIST` and those are the only metrics we capture. We don't want to accidentally record other metrics
//!   recorded by external libraries that could leak more information than we want to expose.
//! * The Histogram we use is a Cumulative Histogram. Meaning that a value is added to each bucket that the sample is smaller than.
//!   This seems to be the way Prometheus expects it
//! * At the moment, with the Histogram we only support one type of bucket which is a bucket for timings in milliseconds.
//!

const METRIC_TARGET: &str = "qe_metrics";
const METRIC_COUNTER: &str = "counter";
const METRIC_GAUGE: &str = "gauge";
const METRIC_HISTOGRAM: &str = "histogram";
const METRIC_DESCRIPTION: &str = "description";

mod common;
mod formatters;
mod recorder;
mod registry;

use recorder::*;
pub use registry::MetricRegistry;

// At the moment the histogram is only used for timings. So the bounds are hard coded here
// The buckets are for ms
pub(crate) const HISTOGRAM_BOUNDS: [f64; 13] = [
    0.0, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0,
];
// We need a list of acceptable metrics we want to expose, we don't want to accidentally expose metrics
// that a different library have or unintended information
const ACCEPT_LIST: &'static [&'static str] = &[
    "pool.active_connections",
    "pool.idle_connections",
    "pool.wait_count",
    "pool.wait_duration",
];

pub fn set_recorder() {
    let recorder = MetricRecorder::default();
    metrics::set_boxed_recorder(Box::new(recorder)).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use metrics::{
        absolute_counter, decrement_gauge, describe_counter, describe_gauge, describe_histogram, gauge, histogram,
        increment_counter, increment_gauge, register_counter, register_gauge, register_histogram,
    };
    use serde_json::json;
    use tracing::instrument::WithSubscriber;
    use tracing::Dispatch;
    use tracing_subscriber::layer::SubscriberExt;

    use once_cell::sync::Lazy;
    use tokio::runtime::Runtime;

    static RT: Lazy<Runtime> = Lazy::new(|| {
        set_recorder();
        Runtime::new().unwrap()
    });

    const TESTING_ACCEPT_LIST: &'static [&'static str] = &[
        "test_counter",
        "another_counter",
        "test_gauge",
        "another_gauge",
        "test_histogram",
        "counter_1",
        "counter_2",
        "gauge_1",
        "gauge_2",
        "histogram_1",
        "histogram_2",
    ];

    #[test]
    fn test_counters() {
        RT.block_on(async {
            let metrics = MetricRegistry::new_with_accept_list(TESTING_ACCEPT_LIST.to_vec());
            let dispatch = Dispatch::new(tracing_subscriber::Registry::default().with(metrics.clone()));
            async {
                let counter1 = register_counter!("test_counter");
                counter1.increment(1);
                increment_counter!("test_counter");
                increment_counter!("test_counter");

                increment_counter!("another_counter");

                let val = metrics.counter_value("test_counter").unwrap();
                assert_eq!(val, 3);

                let val2 = metrics.counter_value("another_counter").unwrap();
                assert_eq!(val2, 1);

                absolute_counter!("test_counter", 5);
                let val3 = metrics.counter_value("test_counter").unwrap();
                assert_eq!(val3, 5);
            }
            .with_subscriber(dispatch)
            .await;
        });
    }

    #[test]
    fn test_gauges() {
        RT.block_on(async {
            let metrics = MetricRegistry::new_with_accept_list(TESTING_ACCEPT_LIST.to_vec());
            let dispatch = Dispatch::new(tracing_subscriber::Registry::default().with(metrics.clone()));
            async {
                let gauge1 = register_gauge!("test_gauge");
                gauge1.increment(1.0);
                increment_gauge!("test_gauge", 1.0);
                increment_gauge!("test_gauge", 1.0);
                increment_gauge!("another_gauge", 1.0);

                let val = metrics.gauge_value("test_gauge").unwrap();
                assert_eq!(val, 3.0);

                let val2 = metrics.gauge_value("another_gauge").unwrap();
                assert_eq!(val2, 1.0);

                assert_eq!(None, metrics.counter_value("test_gauge"));

                gauge!("test_gauge", 5.0);
                let val3 = metrics.gauge_value("test_gauge").unwrap();
                assert_eq!(val3, 5.0);

                decrement_gauge!("test_gauge", 2.0);
                let val4 = metrics.gauge_value("test_gauge").unwrap();
                assert_eq!(val4, 3.0);
            }
            .with_subscriber(dispatch)
            .await;
        });
    }

    #[test]
    fn test_no_panic_and_ignore_other_traces() {
        RT.block_on(async {
            let metrics = MetricRegistry::new_with_accept_list(TESTING_ACCEPT_LIST.to_vec());
            let dispatch = Dispatch::new(tracing_subscriber::Registry::default().with(metrics.clone()));
            async {
                trace!("a fake trace");

                increment_gauge!("test_gauge", 1.0);
                increment_counter!("test_counter");

                trace!("another fake trace");

                assert_eq!(1.0, metrics.gauge_value("test_gauge").unwrap());
                assert_eq!(1, metrics.counter_value("test_counter").unwrap());
            }
            .with_subscriber(dispatch)
            .await;
        });
    }

    #[test]
    fn test_ignore_non_accepted_metrics() {
        RT.block_on(async {
            let metrics = MetricRegistry::new_with_accept_list(TESTING_ACCEPT_LIST.to_vec());
            let dispatch = Dispatch::new(tracing_subscriber::Registry::default().with(metrics.clone()));
            async {
                increment_gauge!("not_accepted", 1.0);
                increment_gauge!("test_gauge", 1.0);

                assert_eq!(1.0, metrics.gauge_value("test_gauge").unwrap());
                assert_eq!(None, metrics.gauge_value("not_accepted"));
            }
            .with_subscriber(dispatch)
            .await;
        });
    }

    #[test]
    fn test_histograms() {
        RT.block_on(async {
            let metrics = MetricRegistry::new_with_accept_list(TESTING_ACCEPT_LIST.to_vec());
            let dispatch = Dispatch::new(tracing_subscriber::Registry::default().with(metrics.clone()));
            async {
                let hist = register_histogram!("test_histogram");
                hist.record(9.0);

                histogram!("test_histogram", 100.0);
                histogram!("test_histogram", 0.1);

                histogram!("test_histogram", 1999.0);
                histogram!("test_histogram", 3999.0);
                histogram!("test_histogram", 610.0);

                let hist = metrics.histogram_values("test_histogram").unwrap();
                let expected: Vec<(f64, u64)> = Vec::from([
                    (0.0, 0),
                    (1.0, 1),
                    (2.0, 1),
                    (5.0, 1),
                    (10.0, 2),
                    (20.0, 2),
                    (50.0, 2),
                    (100.0, 3),
                    (200.0, 3),
                    (500.0, 3),
                    (1000.0, 4),
                    (2000.0, 5),
                    (5000.0, 6),
                ]);

                assert_eq!(hist.buckets(), expected);
            }
            .with_subscriber(dispatch)
            .await;
        });
    }

    #[test]
    fn test_labels() {
        RT.block_on(async {
            let metrics = MetricRegistry::new_with_accept_list(TESTING_ACCEPT_LIST.to_vec());
            let dispatch = Dispatch::new(tracing_subscriber::Registry::default().with(metrics.clone()));
            async {
                let hist = register_histogram!("test_histogram", "label" => "one", "two" => "another");
                hist.record(9.0);
            }
            .with_subscriber(dispatch)
            .await;
        });
    }

    #[test]
    fn test_set_and_read_descriptions() {
        RT.block_on(async {
            let metrics = MetricRegistry::new_with_accept_list(TESTING_ACCEPT_LIST.to_vec());
            let dispatch = Dispatch::new(tracing_subscriber::Registry::default().with(metrics.clone()));
            async {
                describe_counter!("test_counter", "This is a counter");

                let descriptions = metrics.get_descriptions();
                let description = descriptions.get("test_counter").unwrap();

                assert_eq!("This is a counter", description);

                describe_gauge!("test_gauge", "This is a gauge");

                let descriptions = metrics.get_descriptions();
                let description = descriptions.get("test_gauge").unwrap();

                assert_eq!("This is a gauge", description);

                describe_histogram!("test_histogram", "This is a hist");

                let descriptions = metrics.get_descriptions();
                let description = descriptions.get("test_histogram").unwrap();
                assert_eq!("This is a hist", description);
            }
            .with_subscriber(dispatch)
            .await;
        });
    }

    #[test]
    fn test_to_json() {
        RT.block_on(async {
            let metrics = MetricRegistry::new_with_accept_list(TESTING_ACCEPT_LIST.to_vec());
            let dispatch = Dispatch::new(tracing_subscriber::Registry::default().with(metrics.clone()));
            async {
                let empty = json!({
                    "counters": [],
                    "gauges": [],
                    "histograms": []
                });

                assert_eq!(metrics.to_json(), empty);

                absolute_counter!("counter_1", 4, "label" => "one");
                let _ = describe_counter!("counter_2", "this is a description for counter 2");
                absolute_counter!("counter_2", 2, "label" => "one", "another_label" => "two");

                describe_gauge!("gauge_1", "a description for gauge 1");
                gauge!("gauge_1", 7.0);
                gauge!("gauge_2", 3.0, "label" => "three");

                describe_histogram!("histogram_1", "a description for histogram");
                let hist = register_histogram!("histogram_1", "label" => "one", "hist_two" => "two");
                hist.record(9.0);

                histogram!("histogram_2", 1000.0);

                let json = metrics.to_json();
                let expected = json!({
                    "counters": [{
                        "key": "counter_1",
                        "value": 4,
                        "labels": {"label": "one"},
                        "description": ""
                    },{
                        "key": "counter_2",
                        "value": 2,
                        "labels": {"label": "one", "another_label": "two"},
                        "description": "this is a description for counter 2"
                    }],
                    "gauges": [{
                        "key": "gauge_1",
                        "value": 7.0,
                        "labels": {},
                        "description": "a description for gauge 1"
                    }, {
                        "key": "gauge_2",
                        "value": 3.0,
                        "labels": {"label": "three"},
                        "description": ""
                    }],
                    "histograms": [{
                        "key": "histogram_1",
                        "value":[[0.0,0],[1.0,0],[2.0,0],[5.0,0],[10.0,1],[20.0,1],[50.0,1],[100.0,1],[200.0,1],[500.0,1],[1000.0,1],[2000.0,1],[5000.0,1]],
                        "labels": {"label": "one", "hist_two": "two"},
                        "description": "a description for histogram"
                    }, {
                        "key": "histogram_2",
                        "value":[[0.0,0],[1.0,0],[2.0,0],[5.0,0],[10.0,0],[20.0,0],[50.0,0],[100.0,0],[200.0,0],[500.0,0],[1000.0,1],[2000.0,1],[5000.0,1]],
                        "labels": {},
                        "description": ""
                    }]
                });

                assert_eq!(expected, json);
            }
            .with_subscriber(dispatch)
            .await;
        });
    }
}
