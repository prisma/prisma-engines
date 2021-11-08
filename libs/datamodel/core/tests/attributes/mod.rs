mod arg_parsing;
mod builtin_attributes;
mod constraint_names;
mod constraint_names_negative;
mod constraint_names_positive;
mod default_negative;
mod default_positive;
mod id_negative;
mod id_positive;
mod ignore_negative;
mod ignore_positive;
mod index_negative;
mod index_positive;
mod map_negative;
mod map_positive;
mod relations;
mod unique_negative;
mod unique_positive;
mod updated_at_negative;
mod updated_at_positive;

fn with_postgres_provider(dm: &str) -> String {
    let header = r#"
    datasource test {
            provider = "postgres"
            url = "postgresql://..."
    }
    "#;

    format!("{}\n{}", header, dm)
}

fn with_mysql_provider(dm: &str) -> String {
    let header = r#"
    datasource test {
            provider = "mysql"
            url = "mysql://..."
    }
    "#;

    format!("{}\n{}", header, dm)
}

fn with_mssql_provider(dm: &str) -> String {
    let header = r#"
    datasource test {
            provider = "sqlserver"
            url = "sqlserver://..."
    }
    "#;

    format!("{}\n{}", header, dm)
}
