use itertools::Itertools;

pub fn walk_path<'a>(json: &'a serde_json::Value, path: &[&str]) -> &'a serde_json::Value {
    path.iter().fold(json, |acc, p| &acc[p])
}

pub fn parse_identifier(field: &str, json: &serde_json::Value, path: &[&str]) -> String {
    let mut path_with_field = path.to_vec();
    path_with_field.push(field);

    let value = walk_path(json, &path_with_field).to_string();

    format!("{{ {}: {} }}", field, value)
}

pub fn parse_multi_compound(fields: &[String], arg_name: &str, json: &serde_json::Value, path: &[&str]) -> Vec<String> {
    match walk_path(json, path) {
        serde_json::Value::Array(arr) => arr
            .into_iter()
            .map(|json_val| parse_compound_identifier(fields, arg_name, &json_val, &[]))
            .collect::<Vec<_>>(),
        _ => panic!("array expected"),
    }
}

pub fn parse_multi(field: &str, json: &serde_json::Value, path: &[&str]) -> Vec<String> {
    match walk_path(json, path) {
        serde_json::Value::Array(arr) => arr
            .into_iter()
            .map(|json_val| parse_identifier(field, &json_val, &[]))
            .collect::<Vec<_>>(),
        _ => panic!("array expected"),
    }
}

pub fn parse_compound_identifier(fields: &[String], arg_name: &str, json: &serde_json::Value, path: &[&str]) -> String {
    let field_values = fields.iter().map(|field| {
        let mut json_path = path.to_vec();
        json_path.push(field.as_str());

        walk_path(json, &json_path)
    });
    let arguments = fields
        .iter()
        .zip(field_values)
        .map(|(name, value)| format!("{}: {}", name, value.to_string()))
        .join(",");

    format!(
        "{{
            {arg_name}: {{
              {arguments}
            }}
        }}",
        arg_name = arg_name,
        arguments = arguments
    )
}
