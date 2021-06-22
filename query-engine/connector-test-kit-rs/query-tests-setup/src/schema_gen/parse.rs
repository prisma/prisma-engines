use itertools::Itertools;

pub fn walk_path(json: &serde_json::Value, path: &[String]) -> serde_json::Value {
    path.into_iter().fold(json.clone(), |acc, p| acc[p].clone())
}

pub fn parse_identifier(field: &str, json: &serde_json::Value, path: &[String]) -> String {
    let value = walk_path(json, path).to_string();

    format!("{{ {}: {} }}", field, value)
}

pub fn parse_multi_compound(
    fields: &[String],
    arg_name: &str,
    json: &serde_json::Value,
    path: &[String],
) -> Vec<String> {
    let values = match walk_path(json, path) {
        serde_json::Value::Array(arr) => arr
            .into_iter()
            .map(|json_val| parse_compound_identifier(fields.clone(), arg_name, &json_val, &[]))
            .collect::<Vec<_>>(),
        _ => panic!("array expected"),
    };

    values
}

pub fn parse_multi(field: &str, json: &serde_json::Value, path: &[String]) -> Vec<String> {
    match walk_path(json, path) {
        serde_json::Value::Array(arr) => arr
            .into_iter()
            .map(|json_val| parse_identifier(field.clone(), &json_val, &[]))
            .collect::<Vec<_>>(),
        _ => panic!("array expected"),
    }
}

pub fn parse_compound_identifier(
    fields: &[String],
    arg_name: &str,
    json: &serde_json::Value,
    path: &[String],
) -> String {
    let field_values = fields.iter().map(|field| {
        let mut json_path = path.clone().to_vec();
        json_path.push(field.clone());

        walk_path(json, &json_path)
    });
    let arguments = fields
        .iter()
        .zip(field_values)
        .map(|(name, value)| format!("{}: {}", name, value.to_string()))
        .join(",");

    format!(
        "{{
            {}: {{
              {}
            }}
        }}",
        arg_name, arguments
    )
}
