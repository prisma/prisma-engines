#[macro_export]
macro_rules! assert_approximate_float {
    ($data:expr, $field:expr, $val:expr) => {{
        let val: serde_json::Value = query_tests_setup::walk_json(
            &serde_json::from_str::<serde_json::Value>($data.to_string().as_str()).unwrap(),
            $field,
        )
        .unwrap()
        .to_owned();

        if let serde_json::Value::Number(v) = &val {
            if let Some(v) = v.as_f64() {
                if !float_cmp::approx_eq!(f64, v, $val) {
                    panic!("{v} is not close enough to expected value of {}", $val.to_string());
                }
            } else {
                panic!("Expected a float, got {:?}", val);
            }
        } else {
            panic!("Expected a number, got {:?}", val);
        }
    }};
}
