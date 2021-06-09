use datamodel::{parse_schema, render_datamodel_and_config_to_string};
use indoc::indoc;

#[test]
fn shadow_database_url_round_trips() {
    let schema_str = indoc!(
        r#"
        datasource myds {
          provider          = "postgresql"
          url               = "postgres://"
          shadowDatabaseUrl = env("EMPTY_SHADOW_DB URL_0129")
        }

        model Cat {
          id   Int    @id
          name String
        }
        "#
    );

    let (ref config, ref datamodel) = parse_schema(schema_str).unwrap();
    let rendered = render_datamodel_and_config_to_string(datamodel, config);

    assert_eq!(schema_str, rendered);
}
