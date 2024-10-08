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

use once_cell::sync::Lazy;
use recorder::*;
pub use registry::MetricRegistry;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Once;

pub extern crate metrics;
pub use metrics::{describe_counter, describe_gauge, describe_histogram, gauge, histogram, counter};

// Metrics that we emit from the engines, third party metrics emitted by libraries and that we rename are omitted.
pub const PRISMA_CLIENT_QUERIES_TOTAL: &str = "prisma_client_queries_total"; // counter
pub const PRISMA_DATASOURCE_QUERIES_TOTAL: &str = "prisma_datasource_queries_total"; // counter
pub const PRISMA_CLIENT_QUERIES_ACTIVE: &str = "prisma_client_queries_active"; // gauge
pub const PRISMA_CLIENT_QUERIES_DURATION_HISTOGRAM_MS: &str = "prisma_client_queries_duration_histogram_ms"; // histogram
pub const PRISMA_DATASOURCE_QUERIES_DURATION_HISTOGRAM_MS: &str = "prisma_datasource_queries_duration_histogram_ms"; // histogram

// metrics emitted by the connector pool implementation (mobc) that will be renamed using the `METRIC_RENAMES` map.
const MOBC_POOL_CONNECTIONS_OPENED_TOTAL: &str = "mobc_pool_connections_opened_total"; // counter
const MOBC_POOL_CONNECTIONS_CLOSED_TOTAL: &str = "mobc_pool_connections_closed_total"; // counter
const MOBC_POOL_CONNECTIONS_OPEN: &str = "mobc_pool_connections_open"; // gauge
const MOBC_POOL_CONNECTIONS_BUSY: &str = "mobc_pool_connections_busy"; // gauge
const MOBC_POOL_CONNECTIONS_IDLE: &str = "mobc_pool_connections_idle"; // gauge
const MOBC_POOL_WAIT_COUNT: &str = "mobc_client_queries_wait"; // gauge
const MOBC_POOL_WAIT_DURATION: &str = "mobc_client_queries_wait_histogram_ms"; // histogram

/// Accept list: both first-party (emitted by the query engine) and third-party (emitted) metrics
pub const ACCEPT_LIST: &[&str] = &[
    // first-party
    PRISMA_CLIENT_QUERIES_TOTAL,
    PRISMA_DATASOURCE_QUERIES_TOTAL,
    PRISMA_CLIENT_QUERIES_ACTIVE,
    PRISMA_CLIENT_QUERIES_DURATION_HISTOGRAM_MS,
    PRISMA_DATASOURCE_QUERIES_DURATION_HISTOGRAM_MS,
    // third-party, emitted by mobc
    MOBC_POOL_CONNECTIONS_OPENED_TOTAL,
    MOBC_POOL_CONNECTIONS_CLOSED_TOTAL,
    MOBC_POOL_CONNECTIONS_OPEN,
    MOBC_POOL_CONNECTIONS_BUSY,
    MOBC_POOL_CONNECTIONS_IDLE,
    MOBC_POOL_WAIT_COUNT,
    MOBC_POOL_WAIT_DURATION,
];

/// Map that for any given accepted metric that is emitted by a third-party, in this case only the
/// connection pool library mobc, it points to an internal, accepted metrics name and its description
/// as displayed to users. This is used to rebrand the third-party metrics to accepted, prisma-specific
/// ones.
#[rustfmt::skip]
static METRIC_RENAMES: Lazy<HashMap<&'static str, (&'static str, &'static str)>> = Lazy::new(|| {
    HashMap::from([
        (MOBC_POOL_CONNECTIONS_OPENED_TOTAL, ("prisma_pool_connections_opened_total", "The total number of pool connections opened")),
        (MOBC_POOL_CONNECTIONS_CLOSED_TOTAL, ("prisma_pool_connections_closed_total", "The total number of pool connections closed")),
        (MOBC_POOL_CONNECTIONS_OPEN, ("prisma_pool_connections_open", "The number of pool connections currently open")),
        (MOBC_POOL_CONNECTIONS_BUSY, ("prisma_pool_connections_busy", "The number of pool connections currently executing datasource queries")),
        (MOBC_POOL_CONNECTIONS_IDLE, ("prisma_pool_connections_idle", "The number of pool connections that are not busy running a query")),
        (MOBC_POOL_WAIT_COUNT, ("prisma_client_queries_wait", "The number of datasource queries currently waiting for a free connection")),
        (MOBC_POOL_WAIT_DURATION, ("prisma_client_queries_wait_histogram_ms", "The distribution of the time all datasource queries spent waiting for a free connection")),
    ])
});

pub fn setup() {
    set_recorder();
    initialize_metrics();
}

static METRIC_RECORDER: Once = Once::new();

fn set_recorder() {
    METRIC_RECORDER.call_once(|| metrics::set_global_recorder(MetricRecorder).unwrap());
}

/// Initialize metrics descriptions and values
pub fn initialize_metrics() {
    initialize_metrics_descriptions();
    initialize_metrics_values();
}

/// Describe all first-party metrics that we record in prisma-engines. Metrics recorded by third-parties
/// --like mobc-- are described by such third parties, but ignored, and replaced by the descriptions in the
/// METRICS_RENAMES map.
fn initialize_metrics_descriptions() {
    describe_counter!(
        PRISMA_CLIENT_QUERIES_TOTAL,
        "The total number of Prisma Client queries executed"
    );
    describe_counter!(
        PRISMA_DATASOURCE_QUERIES_TOTAL,
        "The total number of datasource queries executed"
    );
    describe_gauge!(
        PRISMA_CLIENT_QUERIES_ACTIVE,
        "The number of currently active Prisma Client queries"
    );
    describe_histogram!(
        PRISMA_CLIENT_QUERIES_DURATION_HISTOGRAM_MS,
        "The distribution of the time Prisma Client queries took to run end to end"
    );
    describe_histogram!(
        PRISMA_DATASOURCE_QUERIES_DURATION_HISTOGRAM_MS,
        "The distribution of the time datasource queries took to run"
    );
}

/// Initialize all metrics values (first and third-party)
///
/// FIXME: https://github.com/prisma/prisma/issues/21070
/// Histograms are excluded, as their initialization will alter the histogram values.
/// (i.e. histograms don't have a neutral value, like counters or gauges)
fn initialize_metrics_values() {
    counter!(PRISMA_CLIENT_QUERIES_TOTAL).absolute(0);
    counter!(PRISMA_DATASOURCE_QUERIES_TOTAL).absolute(0);
    gauge!(PRISMA_CLIENT_QUERIES_ACTIVE).set(0.0);
    counter!(MOBC_POOL_CONNECTIONS_OPENED_TOTAL).absolute(0);
    counter!(MOBC_POOL_CONNECTIONS_CLOSED_TOTAL).absolute(0);
    gauge!(MOBC_POOL_CONNECTIONS_OPEN).set(0.0);
    gauge!(MOBC_POOL_CONNECTIONS_BUSY).set(0.0);
    gauge!(MOBC_POOL_CONNECTIONS_IDLE).set(0.0);
    gauge!(MOBC_POOL_WAIT_COUNT).set(0.0);
}

// At the moment the histogram is only used for timings. So the bounds are hard coded here
// The buckets are for ms
pub(crate) const HISTOGRAM_BOUNDS: [f64; 10] = [0.0, 1.0, 5.0, 10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0, 50000.0];

#[derive(PartialEq, Eq, Debug, Deserialize)]
pub enum MetricFormat {
    #[serde(alias = "json")]
    Json,
    #[serde(alias = "prometheus")]
    Prometheus,
}

#[cfg(test)]
mod tests {
    use super::*;
    use metrics::{describe_counter, describe_gauge, describe_histogram, gauge, histogram};
    use serde_json::json;
    use std::collections::HashMap;
    use std::time::Duration;
    use tracing::instrument::WithSubscriber;
    use tracing::{trace, Dispatch};
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
                let counter1 = counter!("test_counter");
                counter1.increment(1);
                counter!("test_counter").increment(1);
                counter!("test_counter").increment(1);

                counter!("another_counter").increment(1);

                let val = metrics.counter_value("test_counter").unwrap();
                assert_eq!(val, 3);

                let val2 = metrics.counter_value("another_counter").unwrap();
                assert_eq!(val2, 1);

                counter!("test_counter").absolute(5);
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
                let gauge1 = gauge!("test_gauge");
                gauge1.increment(1.0);
                gauge!("test_gauge").increment(1.0);
                gauge!("test_gauge").increment(1.0);
                gauge!("another_gauge").increment(1.0);

                let val = metrics.gauge_value("test_gauge").unwrap();
                assert_eq!(val, 3.0);

                let val2 = metrics.gauge_value("another_gauge").unwrap();
                assert_eq!(val2, 1.0);

                assert_eq!(None, metrics.counter_value("test_gauge"));

                gauge!("test_gauge").set(5.0);
                let val3 = metrics.gauge_value("test_gauge").unwrap();
                assert_eq!(val3, 5.0);

                gauge!("test_gauge").decrement(2.0);
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

                gauge!("test_gauge").set(1.0);
                counter!("test_counter").increment(1);

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
                gauge!("not_accepted").set(1.0);
                gauge!("test_gauge").set(1.0);

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
                let hist = histogram!("test_histogram");
                hist.record(Duration::from_millis(9));

                histogram!("test_histogram").record(Duration::from_millis(100));
                histogram!("test_histogram").record(Duration::from_millis(1));

                histogram!("test_histogram").record(Duration::from_millis(1999));
                histogram!("test_histogram").record(Duration::from_millis(3999));
                histogram!("test_histogram").record(Duration::from_millis(610));

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

                counter!("counter_1", "label" => "one").absolute(4);
                describe_counter!("counter_2", "this is a description for counter 2");
                counter!("counter_2", "label" => "one", "another_label" => "two").absolute(2);

                describe_gauge!("gauge_1", "a description for gauge 1");
                gauge!("gauge_1").set(7.0);
                gauge!("gauge_2", "label" => "three").set(3.0);

                describe_histogram!("histogram_1", "a description for histogram");
                let hist = histogram!("histogram_1", "label" => "one", "hist_two" => "two");
                hist.record(Duration::from_millis(9));

                histogram!("histogram_2").record(Duration::from_millis(9));
                histogram!("histogram_2").record(Duration::from_millis(1000));
                histogram!("histogram_2").record(Duration::from_millis(40));

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
                let hist = histogram!("test_histogram", "label" => "one", "two" => "another");
                hist.record(Duration::from_millis(9));

                counter!("counter_1").absolute(1);

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
                counter!("counter_1", "label" => "one").absolute(4);
                describe_counter!("counter_2", "this is a description for counter 2");
                counter!("counter_2", "label" => "one", "another_label" => "two").absolute(2);

                describe_gauge!("gauge_1", "a description for gauge 1");
                gauge!("gauge_1").set(7.0);
                gauge!("gauge_2", "label" => "three").set(3.0);

                describe_histogram!("histogram_1", "a description for histogram");
                let hist = histogram!("histogram_1", "label" => "one", "hist_two" => "two");
                hist.record(Duration::from_millis(9));

                histogram!("histogram_2").record(Duration::from_millis(1000));

                let mut global_labels: HashMap<String, String> = HashMap::new();
                global_labels.insert("global_two".to_string(), "two".to_string());
                global_labels.insert("global_one".to_string(), "one".to_string());

                let prometheus = metrics.to_prometheus(global_labels);
                let snapshot = expect_test::expect![[r#"
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

                "#]];

                snapshot.assert_eq(&prometheus);
            }
            .with_subscriber(dispatch)
            .await;
        });
    }
}
