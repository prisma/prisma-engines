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

//TODO(matthias) remove the previewFeature and adjust the tests
fn with_postgres_provider(dm: &str) -> String {
    let header = r#"
    datasource test {
            provider = "postgres"
            url = "postgresql://..."
    }
    
    generator js {
            provider = "prisma-client-js"
            previewFeatures = ["NamedConstraints"]
    }"#;

    format!("{}\n{}", header, dm)
}
