pub use dml::composite_type::*;
pub use dml::datamodel::*;
pub use dml::default_value::*;
pub use dml::field::*;
pub use dml::model::*;
pub use dml::native_type_instance::*;
pub use dml::r#enum::*;
pub use dml::relation_info::*;
pub use dml::scalars::*;
pub use dml::traits::*;

pub use dml::PrismaValue;

/// Find the model mapping to the passed in database name.
pub fn find_model_by_db_name<'a>(datamodel: &'a Datamodel, db_name: &str) -> Option<&'a Model> {
    datamodel
        .models
        .iter()
        .find(|model| model.database_name() == Some(db_name) || model.name == db_name)
}
