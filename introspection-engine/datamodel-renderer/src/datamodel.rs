//! Types related to the _datamodel section_ in the PSL.
//!
//! Includes the `model`, `enum` and `type` definitions.

mod attributes;
mod composite_type;
mod default;
mod enumerator;
mod field_type;
mod model;

pub use composite_type::{CompositeType, CompositeTypeField};
pub use default::DefaultValue;
pub use enumerator::{Enum, EnumVariant};
pub use field_type::FieldType;
pub use model::{IdDefinition, IndexDefinition, IndexFieldInput, IndexFieldOptions, Model, ModelField, Relation};
use psl::dml;
use std::fmt;

/// The PSL data model declaration.
#[derive(Default, Debug)]
pub struct Datamodel<'a> {
    enums: Vec<Enum<'a>>,
    composite_types: Vec<CompositeType<'a>>,
}

impl<'a> Datamodel<'a> {
    /// Create a new empty data model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an enum block to the data model.
    ///
    /// ```ignore
    /// enum Foo { // <
    ///   Bar      // < this
    /// }          // <
    /// ```
    pub fn push_enum(&mut self, r#enum: Enum<'a>) {
        self.enums.push(r#enum);
    }

    /// Add a composite type block to the data model.
    ///
    /// ```ignore
    /// type Address {  // <
    ///   street String // < this
    /// }               // <
    /// ```
    pub fn push_composite_type(&mut self, r#enum: CompositeType<'a>) {
        self.composite_types.push(r#enum);
    }

    /// A throwaway function to help generate a rendering from the DML structures.
    ///
    /// Delete when removing DML.
    pub fn from_dml(datasource: &'a psl::Datasource, dml_data_model: &'a dml::Datamodel) -> Datamodel<'a> {
        let mut data_model = Self::new();

        for dml_ct in dml_data_model.composite_types() {
            data_model.push_composite_type(CompositeType::from_dml(datasource, dml_ct))
        }

        for dml_enum in dml_data_model.enums() {
            data_model.push_enum(Enum::from_dml(dml_enum));
        }

        data_model
    }
}

impl<'a> fmt::Display for Datamodel<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for ct in self.composite_types.iter() {
            writeln!(f, "{ct}")?;
        }

        for r#enum in self.enums.iter() {
            writeln!(f, "{enum}")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;

    #[test]
    fn simple_data_model() {
        let mut traffic_light = Enum::new("TrafficLight");

        traffic_light.push_variant("Red");
        traffic_light.push_variant("Yellow");
        traffic_light.push_variant("Green");

        let mut cat = Enum::new("Cat");
        cat.push_variant("Asleep");
        cat.push_variant("Hungry");

        let mut data_model = Datamodel::new();
        data_model.push_enum(traffic_light);
        data_model.push_enum(cat);

        let expected = expect![[r#"
            enum TrafficLight {
              Red
              Yellow
              Green
            }

            enum Cat {
              Asleep
              Hungry
            }
        "#]];

        let rendered = psl::reformat(&format!("{data_model}"), 2).unwrap();
        expected.assert_eq(&rendered);
    }
}
