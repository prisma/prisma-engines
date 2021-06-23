use itertools::Itertools;

use crate::TestError;

pub fn walk_json<'a>(json: &'a serde_json::Value, path: &[&str]) -> Result<&'a serde_json::Value, TestError> {
    path.iter().try_fold(json, |acc, p| match acc.get(p) {
        Some(val) => Ok(val),
        None => Err(TestError::parse_error(format!(
            "Could not walk the JSON value `{}`. The key `{}` does not exist",
            json.to_string(),
            p
        ))),
    })
}

pub fn parse_identifier(field: &str, json: &serde_json::Value, path: &[&str]) -> Result<String, TestError> {
    let mut path_with_field = path.to_vec();
    path_with_field.push(field);

    let value = walk_json(json, &path_with_field)?.to_string();

    Ok(format!("{{ {}: {} }}", field, value))
}

pub fn parse_many_compounds(
    fields: &[String],
    arg_name: &str,
    json: &serde_json::Value,
    path: &[&str],
) -> Result<Vec<String>, TestError> {
    match walk_json(json, path)? {
        serde_json::Value::Array(arr) => {
            let compound_ids = arr
                .iter()
                .map(|json_val| parse_compound_identifier(fields, arg_name, &json_val, &[]))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(compound_ids)
        }
        x => Err(TestError::parse_error(format!(
            "An array was expected but we found: `{}` instead",
            x.to_string()
        ))),
    }
}

pub fn parse_many_ids(field: &str, json: &serde_json::Value, path: &[&str]) -> Result<Vec<String>, TestError> {
    match walk_json(json, path)? {
        serde_json::Value::Array(arr) => {
            let ids = arr
                .iter()
                .map(|json_val| parse_identifier(field, &json_val, &[]))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(ids)
        }
        x => Err(TestError::parse_error(format!(
            "An array was expected but we found: `{}` instead",
            x.to_string()
        ))),
    }
}

pub fn parse_compound_identifier(
    fields: &[String],
    arg_name: &str,
    json: &serde_json::Value,
    path: &[&str],
) -> Result<String, TestError> {
    let field_values = fields
        .iter()
        .map(|field| {
            let mut json_path = path.to_vec();
            json_path.push(field.as_str());

            walk_json(json, &json_path)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let arguments = fields
        .iter()
        .zip(field_values.iter())
        .map(|(name, value)| format!("{}: {}", name, value.to_string()))
        .join(",");

    Ok(format!(
        "{{
            {arg_name}: {{
              {arguments}
            }}
        }}",
        arg_name = arg_name,
        arguments = arguments
    ))
}
