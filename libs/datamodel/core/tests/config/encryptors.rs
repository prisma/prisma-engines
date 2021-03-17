use crate::common::parse_configuration;

#[test]
fn serialize_encryptors_to_cmf() {
    std::env::set_var("HALLO", "secret-vault-token");

    let schema: &str = r#"
encryptor en1 {
    provider = "vault"
    token = "HALLO"
}

encryptor en2 {
    provider = "vault"
    token = env("HALLO")
}

model User {
    id Int @id
    name String
    ssn String @encrypted(encryptor: en1)
}

"#;

    let expected = r#"[
  {
    "name": "en1",
    "provider": "vault",
    "token": "HALLO",
    "config": {}
  },
  {
    "name": "en2",
    "provider": "vault",
    "token": "secret-vault-token",
    "config": {}
  }
]"#;

    assert_mcf(&schema, &expected);
    std::env::remove_var("HALLO")
}

fn assert_mcf(schema: &str, expected_mcf: &str) {
    let config = parse_configuration(schema);
    let rendered = datamodel::json::mcf::encryptors_to_json_value(&config.encryptors).to_string();

    print!("{}", &expected_mcf);

    assert_eq_json(&rendered, expected_mcf);
}

fn assert_eq_json(a: &str, b: &str) {
    let json_a: serde_json::Value = serde_json::from_str(a).expect("The String a was not valid JSON.");
    let json_b: serde_json::Value = serde_json::from_str(b).expect("The String b was not valid JSON.");

    assert_eq!(json_a, json_b);
}
