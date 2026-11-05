use crate::common::*;
use psl::parser_database::ReferentialAction::SetDefault;

fn set_default_schema(provider: &str, mode: &str, arity: &str, default: &str, actions: &str) -> String {
    formatdoc! {r#"
        datasource db {{
          provider = "{provider}"
          {mode}
        }}

        model Parent {{
          id Int @id
          children Child[]
        }}

        model Child {{
          id Int @id
          parentId Int{arity} {default}
          parent Parent{arity} @relation(fields: [parentId], references: [id], {actions})
          @@index([parentId])
        }}
    "#}
}

#[test]
fn set_default_has_no_unsupported_warning_for_database_foreign_keys() {
    for provider in ["kingbase-mysql", "kingbase-oracle"] {
        for mode in ["", "relationMode = \"foreignKeys\""] {
            for arity in ["", "?"] {
                for default in ["", "@default(1)"] {
                    for actions in [
                        "onDelete: SetDefault",
                        "onUpdate: SetDefault",
                        "onDelete: SetDefault, onUpdate: SetDefault",
                    ] {
                        let source = set_default_schema(provider, mode, arity, default, actions);
                        let schema = psl::validate_without_extensions(source.clone().into());
                        assert!(
                            schema.diagnostics.errors().is_empty(),
                            "{source}\n{:?}",
                            schema.diagnostics
                        );
                        // assert_valid alone would miss the old false warning.
                        assert!(
                            schema.diagnostics.warnings().is_empty(),
                            "{source}\n{:?}",
                            schema.diagnostics
                        );
                        let relation = schema.assert_has_model("Child").assert_has_relation_field("parent");
                        if actions.contains("onDelete") {
                            relation.assert_relation_delete_strategy(SetDefault);
                        }
                        if actions.contains("onUpdate") {
                            relation.assert_relation_update_strategy(SetDefault);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn set_default_stays_rejected_for_emulated_relations() {
    for (actions, error_count) in [
        ("onDelete: SetDefault", 1),
        ("onUpdate: SetDefault", 1),
        ("onDelete: SetDefault, onUpdate: SetDefault", 2),
    ] {
        let source = set_default_schema(
            "kingbase-mysql",
            "relationMode = \"prisma\"",
            "?",
            "@default(1)",
            actions,
        );
        let schema = psl::validate_without_extensions(source.clone().into());
        assert_eq!(
            schema.diagnostics.errors().len(),
            error_count,
            "{source}\n{:?}",
            schema.diagnostics
        );
        assert!(
            schema
                .diagnostics
                .errors()
                .iter()
                .all(|error| error.message().contains("Invalid referential action: `SetDefault`"))
        );
        assert!(
            schema.diagnostics.warnings().is_empty(),
            "{source}\n{:?}",
            schema.diagnostics
        );
    }
}

#[test]
fn mysql_still_warns_about_native_set_default() {
    for mode in ["", "relationMode = \"foreignKeys\""] {
        for (actions, warning_count) in [
            ("onDelete: SetDefault", 1),
            ("onUpdate: SetDefault", 1),
            ("onDelete: SetDefault, onUpdate: SetDefault", 2),
        ] {
            let source = set_default_schema("mysql", mode, "?", "@default(1)", actions);
            let schema = psl::validate_without_extensions(source.clone().into());
            assert!(
                schema.diagnostics.errors().is_empty(),
                "{source}\n{:?}",
                schema.diagnostics
            );
            assert_eq!(
                schema.diagnostics.warnings().len(),
                warning_count,
                "{source}\n{:?}",
                schema.diagnostics
            );
            assert!(schema.diagnostics.warnings().iter().all(|warning| {
                warning
                    .message()
                    .starts_with("MySQL does not actually support the `SetDefault` referential action")
            }));
        }
    }
}
