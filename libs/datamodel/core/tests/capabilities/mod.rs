use crate::common::ErrorAsserts;
use crate::common::*;

#[test]
fn enums_must_only_be_supported_if_all_specified_providers_support_them() {
    // Postgres and MySQL support enums but SQLite doesn't.
    test_enum_support(&["postgres", "sqlite", "mysql"], true);
    test_enum_support(&["postgres", "sqlite"], true);
    test_enum_support(&["postgres", "mysql"], false);
    test_enum_support(&["postgres"], false);

    test_enum_support(&["mysql", "sqlite", "postgres"], true);
    test_enum_support(&["mysql", "sqlite"], true);
    test_enum_support(&["mysql", "postgres"], false);
    test_enum_support(&["mysql"], false);

    test_enum_support(&["sqlite", "mysql", "postgres"], true);
    test_enum_support(&["sqlite", "mysql"], true);
    test_enum_support(&["sqlite", "postgres"], true);
    test_enum_support(&["sqlite"], true);
}

fn test_enum_support(providers: &[&str], must_error: bool) {
    let dml = r#"
    model Todo {
      id     Int    @id
      status Status
    }
    
    enum Status {
      DONE
      NOT_DONE
    }  
    "#;

    let error_msg =
        "Error validating: You defined the enum `Status`. But the current connector does not support enums.";
    test_capability_support(providers, must_error, dml, error_msg);
}

#[test]
fn scalar_lists_must_only_be_supported_if_all_specified_providers_support_them() {
    // Only Postgres supports scalar lists.
    test_scalar_list_support(&["postgres", "sqlite", "mysql"], true);
    test_scalar_list_support(&["postgres", "sqlite"], true);
    test_scalar_list_support(&["postgres", "mysql"], true);
    test_scalar_list_support(&["postgres"], false);

    test_scalar_list_support(&["mysql", "sqlite", "postgres"], true);
    test_scalar_list_support(&["mysql", "sqlite"], true);
    test_scalar_list_support(&["mysql", "postgres"], true);
    test_scalar_list_support(&["mysql"], true);

    test_scalar_list_support(&["sqlite", "mysql", "postgres"], true);
    test_scalar_list_support(&["sqlite", "mysql"], true);
    test_scalar_list_support(&["sqlite", "postgres"], true);
    test_scalar_list_support(&["sqlite"], true);
}

fn test_scalar_list_support(providers: &[&str], must_error: bool) {
    let dml = r#"
    model Todo {
      id     Int      @id
      tags   String[]
    }   
    "#;

    let error_msg = "Field \"tags\" in model \"Todo\" can\'t be a list. The current connector does not support lists of primitive types.";
    test_capability_support(providers, must_error, dml, error_msg);
}

#[test]
fn json_must_only_be_supported_if_all_specified_providers_support_them() {
    // Only Postgres and MySQL support JSON.
    test_json_support(&["postgres", "sqlite", "mysql"], true);
    test_json_support(&["postgres", "sqlite"], true);
    test_json_support(&["postgres", "mysql"], false);
    test_json_support(&["postgres"], false);

    test_json_support(&["mysql", "sqlite", "postgres"], true);
    test_json_support(&["mysql", "sqlite"], true);
    test_json_support(&["mysql", "postgres"], false);
    test_json_support(&["mysql"], false);

    test_json_support(&["sqlite", "mysql", "postgres"], true);
    test_json_support(&["sqlite", "mysql"], true);
    test_json_support(&["sqlite", "postgres"], true);
    test_json_support(&["sqlite"], true);
}

fn test_json_support(providers: &[&str], must_error: bool) {
    let dml = r#"
    model Todo {
      id     Int      @id
      json   Json
    }   
    "#;

    let error_msg = "Error validating field `json` in model `Todo`: Field `json` in model `Todo` can\'t be of type Json. The current connector does not support the Json type.";
    test_capability_support(providers, must_error, dml, error_msg);
}

#[test]
fn relations_over_non_unique_criteria_must_only_be_supported_if_all_specified_providers_support_them() {
    // Only MySQL supports that.
    test_relations_over_non_unique_criteria_support(&["postgres", "sqlite", "mysql"], true);
    test_relations_over_non_unique_criteria_support(&["postgres", "sqlite"], true);
    test_relations_over_non_unique_criteria_support(&["postgres", "mysql"], true);
    test_relations_over_non_unique_criteria_support(&["postgres"], true);

    test_relations_over_non_unique_criteria_support(&["mysql", "sqlite", "postgres"], true);
    test_relations_over_non_unique_criteria_support(&["mysql", "sqlite"], true);
    test_relations_over_non_unique_criteria_support(&["mysql", "postgres"], true);
    test_relations_over_non_unique_criteria_support(&["mysql"], false);

    test_relations_over_non_unique_criteria_support(&["sqlite", "mysql", "postgres"], true);
    test_relations_over_non_unique_criteria_support(&["sqlite", "mysql"], true);
    test_relations_over_non_unique_criteria_support(&["sqlite", "postgres"], true);
    test_relations_over_non_unique_criteria_support(&["sqlite"], true);
}

fn test_relations_over_non_unique_criteria_support(providers: &[&str], must_error: bool) {
    let dml = r#"
    model Todo {
      id           Int    @id
      assigneeName String
      assignee     User   @relation(fields: assigneeName, references: name) 
    }
    
    model User {
      id   Int    @id
      name String
    }    
    "#;

    let error_msg = "Error validating: The argument `references` must refer to a unique criteria in the related model `User`. But it is referencing the following fields that are not a unique criteria: name";
    test_capability_support(providers, must_error, dml, error_msg);
}

fn test_capability_support(providers: &[&str], must_error: bool, datamodel: &str, error_msg: &str) {
    let provider_strings: Vec<_> = providers.iter().map(|x| format!("\"{}\"", x)).collect();
    let first_provider = providers.first().unwrap();
    let protocol = match first_provider {
        &"sqlite" => "file:",
        x => x,
    };
    let dml = format!(
        r#"
    datasource db {{
      provider = [{provider_strings}]
      url = "{url}"
    }}
    
    {datamodel}    
    "#,
        provider_strings = provider_strings.join(","),
        url = format!("{}://", protocol),
        datamodel = datamodel,
    );

    if must_error {
        parse_error(&dml).assert_is_message(error_msg);
    } else {
        parse(&dml);
    }
}
