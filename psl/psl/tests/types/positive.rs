use crate::common::*;
use psl::builtin_connectors::{KnownPostgresType, MySqlType, PostgresType};

#[test]
fn should_be_able_to_handle_native_type_combined_with_default_autoincrement_attribute() {
    let dml = r#"
        datasource db {
            provider        = "postgres"
        }

        model User {
            id   Int @id
            name Int    @default(autoincrement()) @db.SmallInt
        }

    "#;

    let datamodel = psl::parse_schema_without_extensions(dml).unwrap();

    let user_model = datamodel.assert_has_model("User");

    let field = user_model.assert_has_scalar_field("name");
    field.assert_default_value().assert_autoincrement();
    field.assert_native_type(datamodel.connector, &PostgresType::Known(KnownPostgresType::SmallInt));

    for (provider, native_type) in [("kingbase-mysql", "SmallInt"), ("kingbase-oracle", "Number(5, 0)")] {
        let dml = formatdoc! {r#"
            datasource db {{
              provider = "{provider}"
            }}
            model User {{
              id Int @id
              name Int @unique @default(autoincrement()) @db.{native_type}
            }}
        "#};
        let datamodel = parse_schema(&dml);
        let field = datamodel.assert_has_model("User").assert_has_scalar_field("name");
        field.assert_default_value().assert_autoincrement();
        match provider {
            "kingbase-mysql" => field.assert_native_type(
                datamodel.connector,
                &psl::builtin_connectors::KingbaseMySqlType::SmallInt,
            ),
            _ => field.assert_native_type(
                datamodel.connector,
                &psl::builtin_connectors::KingbaseOracleType::Number(
                    psl::builtin_connectors::KingbaseOracleNumberArguments::PrecisionAndScale(5, 0),
                ),
            ),
        };
    }
}

#[test]
fn should_be_able_to_handle_native_type_combined_with_default_attribute() {
    let dml = r#"
        datasource db {
            provider        = "mysql"
        }

        model User {
            id    Int      @id
            test  Decimal  @default(1.00) @db.Decimal(8, 2)
        }
    "#;

    let datamodel = psl::parse_schema_without_extensions(dml).unwrap();
    let user_model = datamodel.assert_has_model("User");

    user_model
        .assert_has_scalar_field("test")
        .assert_native_type(datamodel.connector, &MySqlType::Decimal(Some((8, 2))));

    for provider in ["kingbase-mysql", "kingbase-oracle"] {
        let dml = dml.replace("\"mysql\"", &format!("\"{provider}\""));
        let dml = if provider == "kingbase-oracle" {
            dml.replace("@db.Decimal", "@db.Number")
        } else {
            dml
        };
        let datamodel = parse_schema(&dml);
        let field = datamodel.assert_has_model("User").assert_has_scalar_field("test");
        match provider {
            "kingbase-mysql" => field.assert_native_type(
                datamodel.connector,
                &psl::builtin_connectors::KingbaseMySqlType::Decimal(Some((8, 2))),
            ),
            _ => field.assert_native_type(
                datamodel.connector,
                &psl::builtin_connectors::KingbaseOracleType::Number(
                    psl::builtin_connectors::KingbaseOracleNumberArguments::PrecisionAndScale(8, 2),
                ),
            ),
        };
    }
}

#[test]
fn should_handle_type_specifications_on_postgres() {
    let dml = r#"
        datasource pg {
          provider = "postgres"
        }

        model Blog {
            id     Int    @id
            foobar String @pg.VarChar(26)
            foobaz String @pg.VarChar
        }
    "#;

    let datamodel = psl::parse_schema_without_extensions(dml).unwrap();
    let user_model = datamodel.assert_has_model("Blog");

    user_model.assert_has_scalar_field("foobar").assert_native_type(
        datamodel.connector,
        &PostgresType::Known(KnownPostgresType::VarChar(Some(26))),
    );

    user_model.assert_has_scalar_field("foobaz").assert_native_type(
        datamodel.connector,
        &PostgresType::Known(KnownPostgresType::VarChar(None)),
    );

    for provider in ["kingbase-mysql", "kingbase-oracle"] {
        let dml = dml.replace("\"postgres\"", &format!("\"{provider}\""));
        let dml = if provider == "kingbase-mysql" {
            dml.replace("foobaz String @pg.VarChar\n", "foobaz String @pg.VarChar(191)\n")
        } else {
            dml.replace("VarChar", "VarChar2")
        };
        let datamodel = parse_schema(&dml);
        let model = datamodel.assert_has_model("Blog");
        match provider {
            "kingbase-mysql" => {
                model.assert_has_scalar_field("foobar").assert_native_type(
                    datamodel.connector,
                    &psl::builtin_connectors::KingbaseMySqlType::VarChar(26),
                );
                model.assert_has_scalar_field("foobaz").assert_native_type(
                    datamodel.connector,
                    &psl::builtin_connectors::KingbaseMySqlType::VarChar(191),
                );
            }
            _ => {
                model.assert_has_scalar_field("foobar").assert_native_type(
                    datamodel.connector,
                    &psl::builtin_connectors::KingbaseOracleType::VarChar2(Some(26)),
                );
                model.assert_has_scalar_field("foobaz").assert_native_type(
                    datamodel.connector,
                    &psl::builtin_connectors::KingbaseOracleType::VarChar2(None),
                );
            }
        }
    }
}

#[test]
fn should_handle_type_specifications_on_mysql() {
    let dml = r#"
        datasource mys {
          provider = "mysql"
        }

        model Blog {
            id       Int      @id
            smallInt Int      @mys.SmallInt
            foobar   DateTime @mys.DateTime(6)
            fooBool  Boolean  @mys.TinyInt
            fooInt   Int      @mys.TinyInt
        }
    "#;

    let datamodel = psl::parse_schema_without_extensions(dml).unwrap();
    let user_model = datamodel.assert_has_model("Blog");

    user_model
        .assert_has_scalar_field("smallInt")
        .assert_native_type(datamodel.connector, &MySqlType::SmallInt);

    user_model
        .assert_has_scalar_field("foobar")
        .assert_native_type(datamodel.connector, &MySqlType::DateTime(Some(6)));

    user_model
        .assert_has_scalar_field("fooBool")
        .assert_native_type(datamodel.connector, &MySqlType::TinyInt);

    user_model
        .assert_has_scalar_field("fooInt")
        .assert_native_type(datamodel.connector, &MySqlType::TinyInt);

    for provider in ["kingbase-mysql", "kingbase-oracle"] {
        let dml = dml.replace("\"mysql\"", &format!("\"{provider}\""));
        let dml = if provider == "kingbase-oracle" {
            dml.replace("SmallInt", "Number(5, 0)")
                .replace("DateTime(6)", "Timestamp(6)")
                .replace("fooBool  Boolean  @mys.TinyInt", "fooBool  Boolean  @mys.Boolean")
        } else {
            dml
        };
        let datamodel = parse_schema(&dml);
        let model = datamodel.assert_has_model("Blog");
        match provider {
            "kingbase-mysql" => {
                use psl::builtin_connectors::KingbaseMySqlType::*;
                model
                    .assert_has_scalar_field("smallInt")
                    .assert_native_type(datamodel.connector, &SmallInt);
                model
                    .assert_has_scalar_field("foobar")
                    .assert_native_type(datamodel.connector, &DateTime(Some(6)));
                model
                    .assert_has_scalar_field("fooBool")
                    .assert_native_type(datamodel.connector, &TinyInt);
                model
                    .assert_has_scalar_field("fooInt")
                    .assert_native_type(datamodel.connector, &TinyInt);
            }
            _ => {
                use psl::builtin_connectors::{
                    KingbaseOracleNumberArguments::PrecisionAndScale, KingbaseOracleType::*,
                };
                model
                    .assert_has_scalar_field("smallInt")
                    .assert_native_type(datamodel.connector, &Number(PrecisionAndScale(5, 0)));
                model
                    .assert_has_scalar_field("foobar")
                    .assert_native_type(datamodel.connector, &Timestamp(Some(6)));
                model
                    .assert_has_scalar_field("fooBool")
                    .assert_native_type(datamodel.connector, &Boolean);
                model
                    .assert_has_scalar_field("fooInt")
                    .assert_native_type(datamodel.connector, &TinyInt);
            }
        }
    }
}
