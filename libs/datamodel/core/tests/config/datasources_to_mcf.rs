use pretty_assertions::assert_eq;

#[test]
fn serialize_builtin_sources_to_dmmf() {
    std::env::set_var("pg2", "postgresql://localhost/postgres2");

    let schema1 = r#"datasource pg1 {
            provider = "postgresql"
            url = "postgresql://localhost/postgres1"
        }"#;

    let expected_dmmf_1 = r#"[
  {
    "name": "pg1",
    "provider": ["postgresql"],
    "activeProvider": "postgresql",
    "url": {
      "fromEnvVar": null,
      "value": "postgresql://localhost/postgres1"
    }
  }
]"#;

    assert_rendered_mcf(schema1, expected_dmmf_1);

    let schema2 = r#"datasource pg2 {
            provider = ["sqlite", "postgresql"]
            url = env("pg2")
        }"#;

    let expected_dmmf_2 = r#"[
  {
    "name": "pg2",
    "provider": ["sqlite", "postgresql"],
    "activeProvider": "postgresql",
    "url": {
      "fromEnvVar": "pg2",
      "value": "postgresql://localhost/postgres2"
    }
  }
]"#;
    assert_rendered_mcf(schema2, expected_dmmf_2);

    let schema3 = r#"datasource sqlite1 {
            provider = "sqlite"
            url = "file:file.db"
        }"#;

    let expected_dmmf_3 = r#"[
  {
    "name": "sqlite1",
    "provider": ["sqlite"],
    "activeProvider": "sqlite",
    "url": {
      "fromEnvVar": null,
      "value": "file:file.db"
    }
  }
]"#;

    assert_rendered_mcf(schema3, expected_dmmf_3);

    let schema4 = r#"datasource mysql1 {
            provider = "mysql"
            url = "mysql://localhost"
        }"#;
    let expected_dmmf_4 = r#"[
  {
    "name": "mysql1",
    "provider": ["mysql"],
    "activeProvider": "mysql",
    "url": {
      "fromEnvVar": null,
      "value": "mysql://localhost"
    }
  }
]"#;

    assert_rendered_mcf(schema4, expected_dmmf_4);
}

fn assert_rendered_mcf(schema: &str, expected_dmmf: &str) {
    let config = datamodel::parse_configuration(schema).unwrap();
    let rendered = datamodel::json::mcf::render_sources_to_json(&config.datasources);

    print!("{}", &rendered);

    assert_eq_json(&rendered, expected_dmmf);
}

fn assert_eq_json(a: &str, b: &str) {
    let json_a: serde_json::Value = serde_json::from_str(a).expect("The String a was not valid JSON.");
    let json_b: serde_json::Value = serde_json::from_str(b).expect("The String b was not valid JSON.");

    assert_eq!(json_a, json_b);
}
