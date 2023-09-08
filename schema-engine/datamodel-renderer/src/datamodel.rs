//! Types related to the _datamodel section_ in the PSL.
//!
//! Includes the `model`, `enum` and `type` definitions.

mod attributes;
mod composite_type;
mod default;
mod enumerator;
mod field;
mod field_type;
mod index;
mod model;
mod view;

pub use composite_type::CompositeType;
pub use default::DefaultValue;
pub use enumerator::{Enum, EnumVariant};
pub use field::Field;
pub use field_type::FieldType;
pub use index::{IdDefinition, IdFieldDefinition, IndexDefinition, IndexFieldInput, IndexOps, UniqueFieldAttribute};
pub use model::{Model, Relation};
pub use view::View;

use std::fmt;

/// The PSL data model declaration.
#[derive(Default, Debug)]
pub struct Datamodel<'a> {
    models: Vec<Model<'a>>,
    views: Vec<View<'a>>,
    enums: Vec<Enum<'a>>,
    composite_types: Vec<CompositeType<'a>>,
}

impl<'a> Datamodel<'a> {
    /// Create a new empty data model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a model block to the data model.
    ///
    /// ```ignore
    /// model Foo {  // <
    ///   id Int @id // < this
    /// }            // <
    /// ```
    pub fn push_model(&mut self, model: Model<'a>) {
        self.models.push(model);
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

    /// Add a view block to the data model.
    ///
    /// ```ignore
    /// view Foo {   // <
    ///   id Int @id // < this
    /// }            // <
    /// ```
    pub fn push_view(&mut self, view: View<'a>) {
        self.views.push(view);
    }

    /// Add a composite type block to the data model.
    ///
    /// ```ignore
    /// type Address {  // <
    ///   street String // < this
    /// }               // <
    /// ```
    pub fn push_composite_type(&mut self, composite_type: CompositeType<'a>) {
        self.composite_types.push(composite_type);
    }

    /// True if the render output would be an empty string.
    pub fn is_empty(&self) -> bool {
        self.models.is_empty() && self.enums.is_empty() && self.composite_types.is_empty() && self.views.is_empty()
    }
}

impl<'a> fmt::Display for Datamodel<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for ct in self.composite_types.iter() {
            writeln!(f, "{ct}")?;
        }

        for model in self.models.iter() {
            writeln!(f, "{model}")?;
        }

        for view in self.views.iter() {
            writeln!(f, "{view}")?;
        }

        for r#enum in self.enums.iter() {
            writeln!(f, "{enum}")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::value::Function;

    use super::*;
    use expect_test::expect;

    #[test]
    fn simple_data_model() {
        let mut data_model = Datamodel::new();

        let mut composite = CompositeType::new("Address");
        let field = Field::new("street", "String");
        composite.push_field(field);

        data_model.push_composite_type(composite);

        let mut model = Model::new("User");

        let mut field = Field::new("id", "Int");
        field.id(IdFieldDefinition::default());

        let dv = DefaultValue::function(Function::new("autoincrement"));
        field.default(dv);

        model.push_field(field);
        data_model.push_model(model);

        let mut traffic_light = Enum::new("TrafficLight");

        traffic_light.push_variant("Red");
        traffic_light.push_variant("Yellow");
        traffic_light.push_variant("Green");

        data_model.push_enum(traffic_light);

        let mut cat = Enum::new("Cat");
        cat.push_variant("Asleep");
        cat.push_variant("Hungry");

        data_model.push_enum(cat);

        let mut view = View::new("Meow");
        let mut field = Field::new("id", "Int");
        field.id(IdFieldDefinition::default());

        view.push_field(field);

        data_model.push_view(view);

        let expected = expect![[r#"
            type Address {
              street String
            }

            model User {
              id Int @id @default(autoincrement())
            }

            view Meow {
              id Int @id
            }

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
