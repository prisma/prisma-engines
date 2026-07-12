use query_template::{Fragment, PlaceholderFormat, QueryTemplate};

struct Dummy {}

#[test]
fn query_template_formatting_numbered() {
    let pf = PlaceholderFormat {
        prefix: "$",
        has_numbering: true,
    };

    let qt = new_query_template(pf.clone());
    assert_eq!(qt.to_string(), "SELECT * FROM users WHERE id = $1 LIMIT 1");
    assert_eq!(qt.to_sql().unwrap(), "SELECT * FROM users WHERE id = $1 LIMIT 1");

    let qt = new_query_template_with_parameter_tuple(pf);
    assert_eq!(qt.to_string(), "SELECT * FROM users WHERE id = $1 AND status IN [$2]");
    assert!(qt.to_sql().is_err());
}

#[test]
fn query_template_formatting_unnumbered() {
    let pf = PlaceholderFormat {
        prefix: "?",
        has_numbering: false,
    };

    let qt = new_query_template(pf.clone());
    assert_eq!(qt.to_string(), "SELECT * FROM users WHERE id = ? LIMIT 1");
    assert_eq!(qt.to_sql().unwrap(), "SELECT * FROM users WHERE id = ? LIMIT 1");

    let qt = new_query_template_with_parameter_tuple(pf);
    assert_eq!(qt.to_string(), "SELECT * FROM users WHERE id = ? AND status IN [?]");
    assert!(qt.to_sql().is_err());
}

fn new_query_template(pf: PlaceholderFormat) -> QueryTemplate<Dummy> {
    let mut qt = new_common_query_template(pf);
    qt.fragments.push(Fragment::StringChunk {
        chunk: " LIMIT 1".to_string(),
    });
    qt
}

fn new_query_template_with_parameter_tuple(pf: PlaceholderFormat) -> QueryTemplate<Dummy> {
    let mut qt = new_common_query_template(pf);
    qt.fragments.push(Fragment::StringChunk {
        chunk: " AND status IN ".to_string(),
    });
    qt.fragments.push(Fragment::ParameterTuple {
        item_prefix: "".into(),
        item_separator: ", ".into(),
        item_suffix: "".into(),
    });
    qt
}

fn new_common_query_template(pf: PlaceholderFormat) -> QueryTemplate<Dummy> {
    let mut qt: QueryTemplate<Dummy> = QueryTemplate::new(pf);
    qt.fragments.push(Fragment::StringChunk {
        chunk: "SELECT * FROM users WHERE id = ".to_string(),
    });
    qt.fragments.push(Fragment::Parameter);
    qt
}
