pub mod builtin_attributes;
pub mod default_negative;
pub mod default_positive;
pub mod id_negative;
pub mod id_positive;
pub mod index_positive;
pub mod map_negative;
pub mod relations;
pub mod unique_negative;
pub mod unique_positive;
pub mod updated_at_negative;
pub mod updated_at_positive;

pub mod arg_parsing;
mod constraint_names;
mod ignore_negative;
mod ignore_positive;
mod index_negative;
mod map_positive;

fn with_named_constraints(dm: &str) -> String {
    let header = r#"
    datasource test {
            provider = "mysql"
            url = "mysql://root:prisma@127.0.0.1:3309/ReproIndexNames?connection_limit=1"
     }
    
     generator js {
            provider = "prisma-client-js"
            previewFeatures = ["NamedConstraints"]
     }"#;

    format!("{}\n{}", header, dm)
}
