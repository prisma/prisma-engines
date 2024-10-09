/// Functions in this module are meant to parse the JSON result of mutation sent to the Query Engine
/// in order to extract the generated id(s).
use crate::TestError;
use itertools::Itertools;

pub fn walk_json<'a>(json: &'a serde_json::Value, path: &[&str]) -> Result<&'a serde_json::Value, TestError> {
    path.iter().try_fold(json, |acc, p| {
        let key = if p.starts_with('[') && p.ends_with(']') {
            let index: String = p.chars().skip(1).take_while(|c| *c != ']').collect();
            let index = index
                .parse::<usize>()
                .map_err(|err| TestError::parse_error(err.to_string()))?;

            acc.get(index)
        } else {
            acc.get(p)
        };

        match key {
            Some(val) => Ok(val),
            None => Err(TestError::parse_error(format!(
                "Could not walk the JSON value `{json}`. The key `{p}` does not exist"
            ))),
        }
    })
}

/// Parses the JSON result of mutation sent to the Query Engine in order to extract the generated id.
/// Returns a string that's already formatted to be included in another query. eg:
/// { "id": "my_fancy_id" }
pub fn parse_id(field: &str, json: &serde_json::Value, path: &[&str], meta: &str) -> Result<String, TestError> {
    let mut path_with_field = path.to_vec();
    path_with_field.push(field);

    let value = walk_json(json, &path_with_field)?.to_string();

    Ok(format!("{{ {field}: {value}, {meta} }}"))
}

/// Parses the JSON result of mutation sent to the Query Engine in order to extract the generated compound ids.
///
/// Returns a string that's already formatted to be included in another query. eg:
/// { "id_1_id_2": { id_1: "my_fancy_id_1", id_2: "my_fancy_id_2" } }
pub fn parse_compound_id(
    fields: &[String],
    arg_name: &str,
    json: &serde_json::Value,
    path: &[&str],
    meta: &str,
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
        .map(|(name, value)| format!("{name}: {value}"))
        .join(",");

    Ok(format!(
        "{{
            {arg_name}: {{
              {arguments}
            }},
            {meta}
        }}"
    ))
}

/// Performs the same extraction as `parse_compound_id` but for an array
pub fn parse_many_compound_ids(
    fields: &[String],
    arg_name: &str,
    json: &serde_json::Value,
    path: &[&str],
) -> Result<Vec<String>, TestError> {
    match walk_json(json, path)? {
        serde_json::Value::Array(arr) => {
            let compound_ids = arr
                .iter()
                .map(|json_val| parse_compound_id(fields, arg_name, json_val, &[], ""))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(compound_ids)
        }
        x => Err(TestError::parse_error(format!(
            "An array was expected but we found: `{x}` instead"
        ))),
    }
}

/// Performs the same extraction as `parse_id` but for an array
pub fn parse_many_ids(field: &str, json: &serde_json::Value, path: &[&str]) -> Result<Vec<String>, TestError> {
    match walk_json(json, path)? {
        serde_json::Value::Array(arr) => {
            let ids = arr
                .iter()
                .map(|json_val| parse_id(field, json_val, &[], ""))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(ids)
        }
        x => Err(TestError::parse_error(format!(
            "An array was expected but we found: `{x}` instead"
        ))),
    }
}
