use prisma_value::PrismaValue;
use sql_migration_tests::test_api::*;
use sql_schema_describer::ColumnTypeFamily;

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn adding_an_unsupported_type_must_work(api: TestApi) {
    let dm = r#"
        model Post {
            id            Int                     @id @default(autoincrement())
            /// This type is currently not supported.
            user_ip  Unsupported("cidr")
            User          User                    @relation(fields: [user_ip], references: [balance])
        }

        model User {
            id            Int                     @id @default(autoincrement())
            /// This type is currently not supported.
            balance       Unsupported("cidr")  @unique
            Post          Post[]
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("Post", |table| {
        table
            .assert_columns_count(2)
            .assert_column("id", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::Int)
            })
            .assert_column("user_ip", |c| {
                c.assert_is_required()
                    .assert_type_family(ColumnTypeFamily::Unsupported("cidr".to_string()))
            })
    });

    api.assert_schema().assert_table("User", |table| {
        table
            .assert_columns_count(2)
            .assert_column("id", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::Int)
            })
            .assert_column("balance", |c| {
                c.assert_is_required()
                    .assert_type_family(ColumnTypeFamily::Unsupported("cidr".to_string()))
            })
    });
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn switching_an_unsupported_type_to_supported_must_work(api: TestApi) {
    let dm1 = r#"
        model Post {
            id            Int                     @id @default(autoincrement())
            user_home  Unsupported("interval")
            user_location  Unsupported("interval")
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("Post", |table| {
        table
            .assert_columns_count(3)
            .assert_column("id", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::Int)
            })
            .assert_column("user_home", |c| {
                c.assert_is_required()
                    .assert_type_family(ColumnTypeFamily::Unsupported("interval".to_string()))
            })
            .assert_column("user_location", |c| {
                c.assert_is_required()
                    .assert_type_family(ColumnTypeFamily::Unsupported("interval".to_string()))
            })
    });

    let dm2 = r#"
        model Post {
            id            Int                     @id @default(autoincrement())
            user_home     String
            user_location String
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("Post", |table| {
        table
            .assert_columns_count(3)
            .assert_column("id", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::Int)
            })
            .assert_column("user_home", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::String)
            })
            .assert_column("user_location", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::String)
            })
    });
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn adding_and_removing_properties_on_unsupported_should_work(api: TestApi) {
    let dm1 = r#"
        model Post {
            id               Int    @id @default(autoincrement())
            user_ip         Unsupported("cidr")
        }

        model Blog {
          id            Int    @id              @default(autoincrement())
          number        Int?                    @default(1)
          bigger_number Int?                    @default(dbgenerated("sqrt((4)::double precision)"))
          point         Unsupported("point")?   @default(dbgenerated("point((0)::double precision, (0)::double precision)"))
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("Post", |table| {
        table
            .assert_columns_count(2)
            .assert_column("id", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::Int)
            })
            .assert_column("user_ip", |c| {
                c.assert_is_required()
                    .assert_type_family(ColumnTypeFamily::Unsupported("cidr".to_string()))
            })
    });

    api.assert_schema().assert_table("Blog", |table| {
        table
            .assert_columns_count(4)
            .assert_column("id", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::Int)
            })
            .assert_column("number", |c| {
                c.assert_is_nullable()
                    .assert_type_family(ColumnTypeFamily::Int)
                    .assert_default_value(&PrismaValue::Int(1))
            })
            .assert_column("bigger_number", |c| {
                c.assert_is_nullable()
                    .assert_type_family(ColumnTypeFamily::Int)
                    .assert_dbgenerated("sqrt((4)::double precision)")
            })
            .assert_column("point", |c| {
                c.assert_is_nullable()
                    .assert_type_family(ColumnTypeFamily::Unsupported("point".to_string()))
                    .assert_dbgenerated("point((0)::double precision, (0)::double precision)")
            })
    });

    let dm2 = r#"
        model Post {
            id            Int                     @id @default(autoincrement())
            user_ip  Unsupported("cidr")?    @unique
        }
    "#;

    api.schema_push_w_datasource(dm2).force(true).send().assert_warnings(&["A unique constraint covering the columns `[user_ip]` on the table `Post` will be added. If there are existing duplicate values, this will fail.".into()]);

    api.assert_schema().assert_table("Post", |table| {
        table
            .assert_columns_count(2)
            .assert_index_on_columns(&["user_ip"], |index| index.assert_is_unique())
            .assert_column("id", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::Int)
            })
            .assert_column("user_ip", |c| {
                c.assert_is_nullable()
                    .assert_type_family(ColumnTypeFamily::Unsupported("cidr".to_string()))
            })
    });

    let dm3 = r#"
        model Post {
            id               Int    @id @default(autoincrement())
            user_ip     Unsupported("cidr") @default(dbgenerated("'10.1.2.3/32'"))
        }
    "#;

    api.schema_push_w_datasource(dm3).send().assert_green();

    api.assert_schema().assert_table("Post", |table| {
        table
            .assert_columns_count(2)
            .assert_column("id", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::Int)
            })
            .assert_column("user_ip", |c| {
                c.assert_is_required()
                    .assert_type_family(ColumnTypeFamily::Unsupported("cidr".to_string()))
                    .assert_dbgenerated("'10.1.2.3/32'::cidr")
            })
    });
}

#[test_connector]
fn using_unsupported_and_ignore_should_work(api: TestApi) {
    let unsupported_type = if api.is_sqlite() {
        "some random string"
    } else if api.is_cockroach() {
        "interval"
    } else if api.is_postgres() {
        "macaddr"
    } else if api.is_mysql() {
        "point"
    } else if api.is_mssql() {
        "money"
    } else {
        unreachable!()
    };

    let dm = &format!(
        r#"
        model UnsupportedModel {{
            field Unsupported("{unsupported_type}")
            @@ignore
        }}
     "#
    );

    api.schema_push_w_datasource(dm).send().assert_green();
}
