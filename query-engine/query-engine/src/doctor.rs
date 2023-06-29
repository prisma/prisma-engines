use query_core::{ArgumentValue, QueryDocument};

pub fn to_prisma_query(b: QueryDocument) -> Option<String> {
    match b {
        QueryDocument::Single(s) => {
            let query = operation_to_prisma_query(&s);
            println!("query: {}", query.clone().unwrap_or("default".to_string()));
            query
        }
        QueryDocument::Multi(_) => None,
    }
}

fn operation_to_prisma_query(s: &query_core::Operation) -> Option<String> {
    match s {
        query_core::Operation::Read(r) => Some(read_to_prisma_query(r)),
        query_core::Operation::Write(_) => None,
    }
}

fn read_to_prisma_query(r: &query_core::Selection) -> String {
    let (op, model) = extract_operation_and_model(r.name());
    format!("prisma.{}.{}({})", model, op, render_arguments(r.arguments()))
}

fn render_arguments(arguments: &[(String, ArgumentValue)]) -> String {
    let mut result = String::new();
    for (i, (key, value)) in arguments.iter().enumerate() {
        if i > 0 {
            result.push_str(", ");
        }
        result.push_str(&format!("{{ {}: {} }}", key, render_value(&value)));
    }
    result
}

fn render_value(val: &ArgumentValue) -> String {
    match val {
        ArgumentValue::Scalar(_) => "?".to_string(),
        ArgumentValue::Object(vo) => {
            let mut result = String::new();
            for (i, (key, value)) in vo.iter().enumerate() {
                if i > 0 {
                    result.push_str(", ");
                }
                result.push_str(&format!("{}: {}", key, render_value(&value)));
            }
            format!("{{ {} }}", result)
        }
        ArgumentValue::List(_) => "?".to_string(),
        ArgumentValue::FieldRef(_) => "?".to_string(),
    }
}

fn extract_operation_and_model(s: &str) -> (String, String) {
    let operations = vec![
        "aggregate",
        "count",
        "findMany",
        "findOne",
        "findFirst",
        "findUnique",
        "findUniqueOrThrow",
        "findFirstOrThrow",
        "groupBy",
    ];

    for op in operations {
        if let Some((op, rest)) = split_string(op, s) {
            return (op.to_string(), downcase(rest.to_string()));
        }
    }
    return (s.to_string(), "$".to_string());
}

fn downcase(input_string: String) -> String {
    let mut chars = input_string.chars();
    if let Some(first_char) = chars.next() {
        let lowercased_first_char = first_char.to_lowercase().to_string();
        let rest = chars.collect::<String>();

        let result = lowercased_first_char + &rest;
        return result;
    }
    return input_string;
}

fn split_string(keyword: &str, input_string: &str) -> Option<(String, String)> {
    if let Some(index) = input_string.find(keyword) {
        let (keyword_found, rest) = input_string.split_at(index + keyword.len());
        Some((String::from(keyword_found), String::from(rest)))
    } else {
        None
    }
}
