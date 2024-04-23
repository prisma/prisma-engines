use serde_json::Value;

pub fn get_counter(json: &Value, name: &str) -> u64 {
    let metric_value = get_metric_value(json, "counters", name);
    metric_value.as_u64().unwrap()
}

pub fn get_gauge(json: &Value, name: &str) -> f64 {
    let metric_value = get_metric_value(json, "gauges", name);
    metric_value.as_f64().unwrap()
}

pub fn get_metric_value(json: &Value, metric_type: &str, name: &str) -> serde_json::Value {
    let metrics = json.get(metric_type).unwrap().as_array().unwrap();
    let metric = metrics
        .iter()
        .find(|metric| metric.get("key").unwrap().as_str() == Some(name))
        .unwrap()
        .as_object()
        .unwrap();

    metric.get("value").unwrap().clone()
}
