use super::common::{Histogram, Metric, MetricValue, Snapshot};
use metrics_exporter_prometheus::formatting::{
    sanitize_description, sanitize_label_key, sanitize_label_value, write_help_line, write_metric_line, write_type_line,
};
use serde_json::Value;
use std::collections::HashMap;

fn create_label_string(labels: &HashMap<String, String>) -> Vec<String> {
    let mut label_string = labels
        .iter()
        .map(|(k, v)| format!("{}=\"{}\"", sanitize_label_key(k), sanitize_label_value(v)))
        .collect::<Vec<String>>();

    // This sort isn't strictly needed but adds a predictable set order of labels which makes testing easier but
    // should also be better for our users
    label_string.sort();
    label_string
}

pub(crate) fn metrics_to_json(snapshot: Snapshot) -> Value {
    let Snapshot {
        counters,
        histograms,
        gauges,
    } = snapshot;

    // For json output we convert the histogram where a value is only recorded in a single bucket
    let mut normalised_histograms = Vec::new();

    for histogram in histograms {
        if let MetricValue::Histogram(histogram_value) = histogram.value {
            let mut prev = 0;
            let buckets = histogram_value
                .buckets
                .iter()
                .cloned()
                .map(|(le, count)| {
                    let new_count = count - prev;
                    prev = count;
                    (le, new_count)
                })
                .collect();

            let new_histogram = Histogram {
                buckets,
                sum: histogram_value.sum,
                count: histogram_value.count,
            };

            normalised_histograms.push(Metric {
                key: histogram.key.clone(),
                labels: histogram.labels.clone(),
                description: histogram.description.clone(),
                value: MetricValue::Histogram(new_histogram),
            });
        }
    }

    let snapshot = Snapshot {
        counters,
        histograms: normalised_histograms,
        gauges,
    };

    serde_json::to_value(snapshot).unwrap()
}

pub(crate) fn metrics_to_prometheus(snapshot: Snapshot) -> String {
    let Snapshot {
        counters,
        histograms,
        gauges,
    } = snapshot;

    let mut output = String::new();

    for counter in counters {
        let desc = sanitize_description(counter.description.as_str());
        write_help_line(&mut output, counter.key.as_str(), desc.as_str());

        write_type_line(&mut output, counter.key.as_str(), "counter");
        let labels = create_label_string(&counter.labels);

        if let MetricValue::Counter(value) = counter.value {
            write_metric_line::<&str, u64>(&mut output, counter.key.as_str(), None, &labels, None, value);
        }
        output.push('\n');
    }

    for gauge in gauges {
        let desc = sanitize_description(gauge.description.as_str());
        write_help_line(&mut output, gauge.key.as_str(), desc.as_str());

        write_type_line(&mut output, gauge.key.as_str(), "gauge");
        let labels = create_label_string(&gauge.labels);

        if let MetricValue::Gauge(value) = gauge.value {
            write_metric_line::<&str, f64>(&mut output, gauge.key.as_str(), None, &labels, None, value);
        }
        output.push('\n');
    }

    for histogram in histograms {
        let desc = sanitize_description(histogram.description.as_str());
        write_help_line(&mut output, histogram.key.as_str(), desc.as_str());

        write_type_line(&mut output, histogram.key.as_str(), "histogram");
        let labels = create_label_string(&histogram.labels);

        if let MetricValue::Histogram(histogram_values) = histogram.value {
            for (le, count) in histogram_values.buckets {
                write_metric_line(
                    &mut output,
                    histogram.key.as_str(),
                    Some("bucket"),
                    &labels,
                    Some(("le", le)),
                    count,
                );
            }

            write_metric_line(
                &mut output,
                histogram.key.as_str(),
                Some("bucket"),
                &labels,
                Some(("le", "+Inf")),
                histogram_values.count,
            );
            write_metric_line::<&str, f64>(
                &mut output,
                histogram.key.as_str(),
                Some("sum"),
                &labels,
                None,
                histogram_values.sum,
            );
            write_metric_line::<&str, u64>(
                &mut output,
                histogram.key.as_str(),
                Some("count"),
                &labels,
                None,
                histogram_values.count,
            );
        }

        output.push('\n');
    }

    output
}
