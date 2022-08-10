//! Query Engine Metrics
//! This crate is responsible for capturing and recording metrics in the Query Engine.
//! Metrics is broken into two parts, `MetricsRecorder` and `MetricsRegistry`, and uses our tracing framework to communicate.
//! An example best explains this system.
//! When the engine boots up, the `MetricRegistry` is added to our tracing as a layer and The MetricRecorder is
//! set as the global metric recorder. When a metric value is recorded `gauge_increment!("my_gauge", 1.0)` is called.
//! The Metric Recorder is called with `register_gauge` and returns a `MetricHandle`. The `MetricHandle` will convert
//! ``gauge_increment!("my_gauge", 1.0)` into a `trace!` message.
//!
//! The trace message is received by the `MetricRegistry` and converted into a `MetricVisitor` with all the information required
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

use metrics::{absolute_counter, describe_counter, describe_gauge, describe_histogram, gauge};
use recorder::*;
pub use registry::MetricRegistry;
use serde::Deserialize;
use std::sync::Once;

pub(crate) const PRISMA_CLIENT_QUERIES_TOTAL: &str = "prisma_client_queries_total";
pub(crate) const PRISMA_CLIENT_QUERIES_HISTOGRAM_MS: &str = "prisma_client_queries_duration_histogram_ms";

// At the moment the histogram is only used for timings. So the bounds are hard coded here
// The buckets are for ms
pub(crate) const HISTOGRAM_BOUNDS: [f64; 10] = [0.0, 1.0, 5.0, 10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0, 50000.0];
// We need a list of acceptable metrics we want to expose, we don't want to accidentally expose metrics
// that a different library have or unintended information
const ACCEPT_LIST: &[&str] = &[
    "prisma_pool_connections_opened_total",
    "prisma_pool_connections_closed_total",
    "prisma_pool_connections_open",
    "prisma_pool_connections_busy",
    "prisma_pool_connections_idle",
    "prisma_client_queries_wait",
    "prisma_client_queries_wait_histogram_ms",
    "prisma_datasource_queries_duration_histogram_ms",
    "prisma_datasource_queries_total",
    "prisma_client_queries_active",
    PRISMA_CLIENT_QUERIES_HISTOGRAM_MS,
    PRISMA_CLIENT_QUERIES_TOTAL,
];

#[derive(PartialEq, Debug, Deserialize)]
pub enum MetricFormat {
    #[serde(alias = "json")]
    Json,
    #[serde(alias = "prometheus")]
    Prometheus,
}

pub fn setup() {
    set_recorder();
    describe_metrics();
}

// Describe all metric here so that every time for create
// a new metric registry for a Query Instance the descriptions
// will be in place
pub fn describe_metrics() {
    describe_counter!(
        "prisma_pool_connections_opened_total",
        "Total number of Pool Connections opened"
    );
    describe_counter!(
        "prisma_pool_connections_closed_total",
        "Total number of Pool Connections closed"
    );
    describe_gauge!(
        "prisma_pool_connections_busy",
        "Number of currently busy Pool Connections (executing a datasource query)"
    );

    describe_gauge!(
        "prisma_pool_connections_idle",
        "Number of currently unused Pool Connections (waiting for the next datasource query to run)"
    );

    describe_gauge!(
        "prisma_client_queries_wait",
        "Number of Prisma Client queries currently waiting for a connection"
    );
    describe_gauge!(
        "prisma_client_queries_active",
        "Number of currently active Prisma Client queries"
    );

    gauge!("prisma_pool_connections_busy", 0.0);
    gauge!("prisma_pool_connections_idle", 0.0);
    gauge!("prisma_client_queries_wait", 0.0);
    gauge!("prisma_client_queries_active", 0.0);

    describe_gauge!(
        "prisma_client_queries_active",
        "Number of currently active Prisma Client queries"
    );

    describe_gauge!(
        "prisma_pool_connections_busy",
        "Number of currently busy Pool Connections (executing a datasource query)"
    );

    describe_gauge!(
        "prisma_pool_connections_idle",
        "Number of currently unused Pool Connections (waiting for the next datasource query to run)"
    );

    describe_gauge!(
        "prisma_client_queries_wait",
        "Number of Prisma Client queries currently waiting for a connection"
    );

    describe_histogram!(
        "prisma_client_queries_wait_histogram_ms",
        "Histogram of the wait time of all Prisma Client Queries in ms"
    );
    describe_histogram!(
        "prisma_datasource_queries_duration_histogram_ms",
        "Histogram of the duration of all executed Datasource Queries in ms"
    );

    describe_histogram!(
        PRISMA_CLIENT_QUERIES_HISTOGRAM_MS,
        "Histogram of the duration of all executed Prisma Client queries in ms"
    );

    describe_counter!(
        "prisma_datasource_queries_total",
        "Total number of Datasource Queries executed"
    );

    describe_counter!(
        PRISMA_CLIENT_QUERIES_TOTAL,
        "Total number of Prisma Client queries executed"
    );

    absolute_counter!("prisma_datasource_queries_total", 0);
    absolute_counter!(PRISMA_CLIENT_QUERIES_TOTAL, 0);
}

static METRIC_RECORDER: Once = Once::new();

fn set_recorder() {
    METRIC_RECORDER.call_once(|| {
        let recorder = MetricRecorder::default();
        metrics::set_boxed_recorder(Box::new(recorder)).unwrap();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use metrics::{
        absolute_counter, decrement_gauge, describe_counter, describe_gauge, describe_histogram, gauge, histogram,
        increment_counter, increment_gauge, register_counter, register_gauge, register_histogram,
    };
    use serde_json::json;
    use std::collections::HashMap;
    use std::time::Duration;
    use tracing::instrument::WithSubscriber;
    use tracing::Dispatch;
    use tracing_subscriber::layer::SubscriberExt;

    use once_cell::sync::Lazy;
    use tokio::runtime::Runtime;

    static RT: Lazy<Runtime> = Lazy::new(|| {
        set_recorder();
        Runtime::new().unwrap()
    });

    const TESTING_ACCEPT_LIST: &[&str] = &[
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
        "test.counter",
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
                hist.record(Duration::from_millis(9));

                histogram!("test_histogram", Duration::from_millis(100));
                histogram!("test_histogram", Duration::from_millis(1));

                histogram!("test_histogram", Duration::from_millis(1999));
                histogram!("test_histogram", Duration::from_millis(3999));
                histogram!("test_histogram", Duration::from_millis(610));

                let hist = metrics.histogram_values("test_histogram").unwrap();
                let expected: Vec<(f64, u64)> = Vec::from([
                    (0.0, 0),
                    (1.0, 1),
                    (5.0, 1),
                    (10.0, 2),
                    (50.0, 2),
                    (100.0, 3),
                    (500.0, 3),
                    (1000.0, 4),
                    (5000.0, 6),
                    (50000.0, 6),
                ]);

                assert_eq!(hist.buckets(), expected);
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

                assert_eq!(metrics.to_json(Default::default()), empty);

                absolute_counter!("counter_1", 4, "label" => "one");
                describe_counter!("counter_2", "this is a description for counter 2");
                absolute_counter!("counter_2", 2, "label" => "one", "another_label" => "two");

                describe_gauge!("gauge_1", "a description for gauge 1");
                gauge!("gauge_1", 7.0);
                gauge!("gauge_2", 3.0, "label" => "three");

                describe_histogram!("histogram_1", "a description for histogram");
                let hist = register_histogram!("histogram_1", "label" => "one", "hist_two" => "two");
                hist.record(Duration::from_millis(9));

                histogram!("histogram_2", Duration::from_millis(9));
                histogram!("histogram_2", Duration::from_millis(1000));
                histogram!("histogram_2", Duration::from_millis(40));

                let json = metrics.to_json(Default::default());
                let expected = json!({
                    "counters":[{
                        "key":"counter_1",
                        "labels":{"label":"one"},
                        "value":4,
                        "description":""
                    },{
                        "key":"counter_2",
                        "labels":{"label":"one","another_label":"two"},
                        "value":2,
                        "description":"this is a description for counter 2"
                    }],
                    "gauges":[{
                        "key":"gauge_1",
                        "labels":{},
                        "value":7.0,
                        "description":"a description for gauge 1"
                    },{
                        "key":"gauge_2",
                        "labels":{"label":"three"},
                        "value":3.0,
                        "description":""
                    }],
                    "histograms":[{
                        "key":"histogram_1",
                        "labels":{"label":"one","hist_two":"two"},
                        "value":{
                            "buckets": [[0.0,0],[1.0,0],[5.0,0],[10.0,1],[50.0,0],[100.0,0],[500.0,0],[1000.0,0],[5000.0,0],[50000.0,0]],
                            "sum":9.0,
                            "count":1
                        },
                        "description":"a description for histogram"},{
                            "key":"histogram_2",
                            "labels":{},
                            "value":{
                                "buckets":[[0.0,0],[1.0,0],[5.0,0],[10.0,1],[50.0,1],[100.0,0],[500.0,0],[1000.0,1],[5000.0,0],[50000.0,0]],
                                "sum":1049.0,
                                "count":3
                            },
                            "description":""
                        }]
                    });

                assert_eq!(json, expected);
            }
            .with_subscriber(dispatch)
            .await;
        });
    }

    #[test]
    fn test_global_and_metric_labels() {
        RT.block_on(async {
            let metrics = MetricRegistry::new_with_accept_list(TESTING_ACCEPT_LIST.to_vec());
            let dispatch = Dispatch::new(tracing_subscriber::Registry::default().with(metrics.clone()));
            async {
                let hist = register_histogram!("test_histogram", "label" => "one", "two" => "another");
                hist.record(Duration::from_millis(9));

                absolute_counter!("counter_1", 1);

                let mut global_labels: HashMap<String, String> = HashMap::new();
                global_labels.insert("global_one".to_string(), "one".to_string());
                global_labels.insert("global_two".to_string(), "two".to_string());

                let json = metrics.to_json(global_labels);

                let expected = json!({
                    "counters":[{
                        "key":"counter_1",
                        "labels":{"global_one":"one","global_two":"two"},
                        "value":1,
                        "description":""
                    }],
                    "gauges":[],
                    "histograms":[{
                        "key":"test_histogram",
                        "labels":{"label":"one","two":"another","global_one":"one","global_two":"two"},
                        "value":{
                            "buckets": [[0.0,0],[1.0,0],[5.0,0],[10.0,1],[50.0,0],[100.0,0],[500.0,0],[1000.0,0],[5000.0,0],[50000.0,0]],
                            "sum": 9.0,
                            "count": 1
                        },
                        "description":""
                    }]
                });
                assert_eq!(expected, json);
            }
            .with_subscriber(dispatch)
            .await;
        });
    }

    #[test]
    fn test_prometheus_format() {
        RT.block_on(async {
            let metrics = MetricRegistry::new_with_accept_list(TESTING_ACCEPT_LIST.to_vec());
            let dispatch = Dispatch::new(tracing_subscriber::Registry::default().with(metrics.clone()));
            async {
                absolute_counter!("counter_1", 4, "label" => "one");
                describe_counter!("counter_2", "this is a description for counter 2");
                absolute_counter!("counter_2", 2, "label" => "one", "another_label" => "two");

                describe_gauge!("gauge_1", "a description for gauge 1");
                gauge!("gauge_1", 7.0);
                gauge!("gauge_2", 3.0, "label" => "three");

                describe_histogram!("histogram_1", "a description for histogram");
                let hist = register_histogram!("histogram_1", "label" => "one", "hist_two" => "two");
                hist.record(Duration::from_millis(9));

                histogram!("histogram_2", Duration::from_millis(1000));

                let mut global_labels: HashMap<String, String> = HashMap::new();
                global_labels.insert("global_two".to_string(), "two".to_string());
                global_labels.insert("global_one".to_string(), "one".to_string());

                let prometheus = metrics.to_prometheus(global_labels);
                let snapshot = expect_test::expect![[r##"
                    # HELP counter_1 
                    # TYPE counter_1 counter
                    counter_1{global_one="one",global_two="two",label="one"} 4

                    # HELP counter_2 this is a description for counter 2
                    # TYPE counter_2 counter
                    counter_2{another_label="two",global_one="one",global_two="two",label="one"} 2

                    # HELP gauge_1 a description for gauge 1
                    # TYPE gauge_1 gauge
                    gauge_1{global_one="one",global_two="two"} 7

                    # HELP gauge_2 
                    # TYPE gauge_2 gauge
                    gauge_2{global_one="one",global_two="two",label="three"} 3

                    # HELP histogram_1 a description for histogram
                    # TYPE histogram_1 histogram
                    histogram_1_bucket{global_one="one",global_two="two",hist_two="two",label="one",le="0"} 0
                    histogram_1_bucket{global_one="one",global_two="two",hist_two="two",label="one",le="1"} 0
                    histogram_1_bucket{global_one="one",global_two="two",hist_two="two",label="one",le="5"} 0
                    histogram_1_bucket{global_one="one",global_two="two",hist_two="two",label="one",le="10"} 1
                    histogram_1_bucket{global_one="one",global_two="two",hist_two="two",label="one",le="50"} 1
                    histogram_1_bucket{global_one="one",global_two="two",hist_two="two",label="one",le="100"} 1
                    histogram_1_bucket{global_one="one",global_two="two",hist_two="two",label="one",le="500"} 1
                    histogram_1_bucket{global_one="one",global_two="two",hist_two="two",label="one",le="1000"} 1
                    histogram_1_bucket{global_one="one",global_two="two",hist_two="two",label="one",le="5000"} 1
                    histogram_1_bucket{global_one="one",global_two="two",hist_two="two",label="one",le="50000"} 1
                    histogram_1_bucket{global_one="one",global_two="two",hist_two="two",label="one",le="+Inf"} 1
                    histogram_1_sum{global_one="one",global_two="two",hist_two="two",label="one"} 9
                    histogram_1_count{global_one="one",global_two="two",hist_two="two",label="one"} 1

                    # HELP histogram_2 
                    # TYPE histogram_2 histogram
                    histogram_2_bucket{global_one="one",global_two="two",le="0"} 0
                    histogram_2_bucket{global_one="one",global_two="two",le="1"} 0
                    histogram_2_bucket{global_one="one",global_two="two",le="5"} 0
                    histogram_2_bucket{global_one="one",global_two="two",le="10"} 0
                    histogram_2_bucket{global_one="one",global_two="two",le="50"} 0
                    histogram_2_bucket{global_one="one",global_two="two",le="100"} 0
                    histogram_2_bucket{global_one="one",global_two="two",le="500"} 0
                    histogram_2_bucket{global_one="one",global_two="two",le="1000"} 1
                    histogram_2_bucket{global_one="one",global_two="two",le="5000"} 1
                    histogram_2_bucket{global_one="one",global_two="two",le="50000"} 1
                    histogram_2_bucket{global_one="one",global_two="two",le="+Inf"} 1
                    histogram_2_sum{global_one="one",global_two="two"} 1000
                    histogram_2_count{global_one="one",global_two="two"} 1

                "##]];

                snapshot.assert_eq(&prometheus);
            }
            .with_subscriber(dispatch)
            .await;
        });
    }
}
