use barrel::types;
use enumflags2::BitFlags;
use indoc::formatdoc;
use introspection_engine_tests::test_api::*;
use introspection_engine_tests::TestResult;
use quaint::connector::SqlFamily;
use test_macros::test_connector;

#[test_connector]
async fn legacy_referential_actions(api: &TestApi) -> TestResult {
    let family = api.sql_family();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("a", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("b", move |t| {
                t.add_column("id", types::primary());
                t.add_column("a_id", types::integer().nullable(false));

                match family {
                    SqlFamily::Mssql => {
                        t.inject_custom(
                            "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES legacy_referential_actions.a(id) ON DELETE NO ACTION ON UPDATE NO ACTION",
                        );
                    }
                    _ => {
                        t.inject_custom(
                            "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES a(id) ON DELETE NO ACTION ON UPDATE NO ACTION",
                        );
                    }
                }
            });
        })
        .await?;

    let extra_index = if api.sql_family().is_mysql() {
        r#"@@index([a_id], name: "asdf")"#
    } else {
        ""
    };

    let expected = formatdoc! {r#"
        model a {{
            id Int @id @default(autoincrement())
            b  b[] 
        }}

        model b {{
            id Int @id @default(autoincrement())
            a_id Int
            a a @relation(fields: [a_id], references: [id])
            {}
        }}
    "#, extra_index};

    api.assert_eq_datamodels(&expected, &api.introspect().await?);

    Ok(())
}

#[test_connector(preview_features("referentialActions"))]
async fn referential_actions(api: &TestApi) -> TestResult {
    let family = api.sql_family();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("a", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("b", move |t| {
                t.add_column("id", types::primary());
                t.add_column("a_id", types::integer().nullable(false));

                match family {
                    SqlFamily::Mssql => {
                        t.inject_custom(
                            "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES referential_actions.a(id) ON DELETE CASCADE ON UPDATE NO ACTION",
                        );
                    }
                    _ => {
                        t.inject_custom(
                            "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES a(id) ON DELETE CASCADE ON UPDATE NO ACTION",
                        );
                    }
                }
            });
        })
        .await?;

    let extra_index = if api.sql_family().is_mysql() {
        r#"@@index([a_id], name: "asdf")"#
    } else {
        ""
    };

    let expected = formatdoc! {r#"
        model a {{
            id Int @id @default(autoincrement())
            b  b[]
        }}

        model b {{
            id Int @id @default(autoincrement())
            a_id Int
            a a @relation(fields: [a_id], references: [id], onDelete: Cascade, onUpdate: NoAction)
            {}
        }}
    "#, extra_index};

    api.assert_eq_datamodels(&expected, &api.introspect().await?);

    Ok(())
}

#[test_connector(tags(Postgres, Mysql, Sqlite), preview_features("referentialActions"))]
async fn default_referential_actions_with_restrict(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("a", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("b", |t| {
                t.add_column("id", types::primary());
                t.add_column("a_id", types::integer().nullable(false));
                t.inject_custom(
                    "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES a(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let extra_index = if api.sql_family().is_mysql() {
        r#"@@index([a_id], name: "asdf")"#
    } else {
        ""
    };

    let expected = formatdoc! {r#"
        model a {{
            id Int @id @default(autoincrement())
            b  b[]
        }}

        model b {{
            id Int @id @default(autoincrement())
            a_id Int
            a a @relation(fields: [a_id], references: [id])
            {}
        }}
    "#, extra_index};

    api.assert_eq_datamodels(&expected, &api.introspect().await?);

    Ok(())
}

#[test_connector(tags(Mssql), preview_features("referentialActions"))]
async fn default_referential_actions_without_restrict(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("a", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("b", |t| {
                t.add_column("id", types::primary());
                t.add_column("a_id", types::integer().nullable(false));
                t.inject_custom(
                    "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES default_referential_actions_without_restrict.a(id) ON DELETE NO ACTION ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let extra_index = if api.sql_family().is_mysql() {
        r#"@@index([a_id], name: "asdf")"#
    } else {
        ""
    };

    let expected = formatdoc! {r#"
        model a {{
            id Int @id @default(autoincrement())
            b  b[]
        }}

        model b {{
            id Int @id @default(autoincrement())
            a_id Int
            a a @relation(fields: [a_id], references: [id])
            {}
        }}
    "#, extra_index};

    api.assert_eq_datamodels(&expected, &api.introspect().await?);

    Ok(())
}

#[test_connector(preview_features("referentialActions"))]
async fn default_optional_actions(api: &TestApi) -> TestResult {
    let family = api.sql_family();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("a", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("b", move |t| {
                t.add_column("id", types::primary());
                t.add_column("a_id", types::integer().nullable(true));

                match family {
                    SqlFamily::Mssql => {
                        t.inject_custom(
                            "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES default_optional_actions.a(id) ON DELETE SET NULL ON UPDATE CASCADE",
                        );
                    }
                    _ => {
                        t.inject_custom(
                            "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES a(id) ON DELETE SET NULL ON UPDATE CASCADE",
                        );
                    }
                }
            });
        })
        .await?;

    let extra_index = if api.sql_family().is_mysql() {
        r#"@@index([a_id], name: "asdf")"#
    } else {
        ""
    };

    let expected = formatdoc! {r#"
        model a {{
            id Int @id @default(autoincrement())
            b  b[]
        }}

        model b {{
            id Int @id @default(autoincrement())
            a_id Int?
            a a? @relation(fields: [a_id], references: [id])
            {}
        }}
    "#, extra_index};

    api.assert_eq_datamodels(&expected, &api.introspect().await?);

    Ok(())
}
