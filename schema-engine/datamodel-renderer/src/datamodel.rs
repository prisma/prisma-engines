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
use psl::SourceFile;
pub use view::View;

use crate::Configuration;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

/// The PSL data model declaration.
#[derive(Default, Debug)]
pub struct Datamodel<'a> {
    models: HashMap<Cow<'a, str>, Vec<Model<'a>>>,
    views: HashMap<Cow<'a, str>, Vec<View<'a>>>,
    enums: HashMap<Cow<'a, str>, Vec<Enum<'a>>>,
    composite_types: HashMap<Cow<'a, str>, Vec<CompositeType<'a>>>,
    configuration: Option<Configuration<'a>>,
    empty_files: HashSet<Cow<'a, str>>,
}

impl<'a> Datamodel<'a> {
    /// Create a new empty data model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an empty file in the data model.
    pub fn create_empty_file(&mut self, file: impl Into<Cow<'a, str>>) {
        self.empty_files.insert(file.into());
    }

    /// Add a model block to the data model.
    ///
    /// ```ignore
    /// model Foo {  // <
    ///   id Int @id // < this
    /// }            // <
    /// ```
    pub fn push_model(&mut self, file: impl Into<Cow<'a, str>>, model: Model<'a>) {
        self.models.entry(file.into()).or_default().push(model);
    }

    /// Add an enum block to the data model.
    ///
    /// ```ignore
    /// enum Foo { // <
    ///   Bar      // < this
    /// }          // <
    /// ```
    pub fn push_enum(&mut self, file: impl Into<Cow<'a, str>>, r#enum: Enum<'a>) {
        self.enums.entry(file.into()).or_default().push(r#enum);
    }

    /// Add a view block to the data model.
    ///
    /// ```ignore
    /// view Foo {   // <
    ///   id Int @id // < this
    /// }            // <
    /// ```
    pub fn push_view(&mut self, file: impl Into<Cow<'a, str>>, view: View<'a>) {
        self.views.entry(file.into()).or_default().push(view);
    }

    /// Add a composite type block to the data model.
    ///
    /// ```ignore
    /// type Address {  // <
    ///   street String // < this
    /// }               // <
    /// ```
    pub fn push_composite_type(&mut self, file: impl Into<Cow<'a, str>>, composite_type: CompositeType<'a>) {
        self.composite_types
            .entry(file.into())
            .or_default()
            .push(composite_type);
    }

    /// True if the render output would be an empty string.
    pub fn is_empty(&self) -> bool {
        self.models.is_empty() && self.enums.is_empty() && self.composite_types.is_empty() && self.views.is_empty()
    }

    /// Renders the datamodel into a list of file names and their content.
    pub fn render(self) -> Vec<(String, SourceFile)> {
        let mut rendered: HashMap<Cow<'a, str>, String> = HashMap::new();

        if let Some(config) = self.configuration {
            for (file, generators) in config.generators {
                let generator_str = rendered.entry(file).or_default();

                for generator in generators {
                    generator_str.push_str(&format!("{generator}\n"));
                }
            }

            for (file, datasources) in config.datasources {
                let datasource_str = rendered.entry(file).or_default();

                for datasource in datasources {
                    datasource_str.push_str(&format!("{datasource}\n"));
                }
            }
        }

        for (file, composite_types) in self.composite_types {
            let composite_type_str = rendered.entry(file).or_default();

            for composite_type in composite_types {
                composite_type_str.push_str(&format!("{composite_type}\n"));
            }
        }

        for (file, models) in self.models {
            let model_str = rendered.entry(file).or_default();

            for model in models {
                model_str.push_str(&format!("{model}\n"));
            }
        }

        for (file, views) in self.views {
            let view_str = rendered.entry(file).or_default();

            for view in views {
                view_str.push_str(&format!("{view}\n"));
            }
        }

        for (file, enums) in self.enums {
            let enum_str = rendered.entry(file).or_default();

            for r#enum in enums {
                enum_str.push_str(&format!("{enum}\n"));
            }
        }

        for empty_file in self.empty_files {
            rendered.entry(empty_file).or_default();
        }

        rendered
            .into_iter()
            .map(|(file, content)| (file.into_owned(), SourceFile::from(content)))
            .collect()
    }

    /// Sets the configuration blocks for a datamodel.
    pub fn set_configuration(&mut self, config: Configuration<'a>) {
        self.configuration = Some(config);
    }
}

#[cfg(test)]
mod tests {
    use crate::value::Function;

    use super::*;
    use expect_test::expect;
    use itertools::Itertools as _;

    fn pretty_render(data_model: Datamodel) -> String {
        let sources = data_model.render();
        let sources = psl::reformat_multiple(sources, 2);

        sources
            .into_iter()
            .sorted_unstable_by_key(|(file_name, _)| file_name.to_owned())
            .map(|(file_name, dm)| format!("// file: {file_name}\n{}", dm.as_str()))
            .join("------\n")
    }

    #[test]
    fn simple_data_model() {
        let file_name = "schema.prisma";
        let mut data_model = Datamodel::new();

        let mut composite = CompositeType::new("Address");
        let field = Field::new("street", "String");
        composite.push_field(field);

        data_model.push_composite_type(file_name.to_string(), composite);

        let mut model = Model::new("User");

        let mut field = Field::new("id", "Int");
        field.id(IdFieldDefinition::default());

        let dv = DefaultValue::function(Function::new("autoincrement"));
        field.default(dv);

        model.push_field(field);
        data_model.push_model(file_name.to_string(), model);

        let mut traffic_light = Enum::new("TrafficLight");

        traffic_light.push_variant("Red");
        traffic_light.push_variant("Yellow");
        traffic_light.push_variant("Green");

        data_model.push_enum(file_name.to_string(), traffic_light);

        let mut cat = Enum::new("Cat");
        cat.push_variant("Asleep");
        cat.push_variant("Hungry");

        data_model.push_enum(file_name.to_string(), cat);

        let mut view = View::new("Meow");
        let mut field = Field::new("id", "Int");
        field.id(IdFieldDefinition::default());

        view.push_field(field);

        data_model.push_view(file_name.to_string(), view);

        let expected = expect![[r#"
            // file: schema.prisma
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
        let rendered = pretty_render(data_model);

        expected.assert_eq(&rendered);
    }

    #[test]
    fn data_model_multi_file() {
        let mut data_model = Datamodel::new();

        let mut composite = CompositeType::new("Address");
        let field = Field::new("street", "String");
        composite.push_field(field);

        data_model.push_composite_type("a.prisma".to_string(), composite);

        let mut model = Model::new("User");

        let mut field = Field::new("id", "Int");
        field.id(IdFieldDefinition::default());

        let dv = DefaultValue::function(Function::new("autoincrement"));
        field.default(dv);

        model.push_field(field);
        data_model.push_model("a.prisma".to_string(), model);

        let mut traffic_light = Enum::new("TrafficLight");

        traffic_light.push_variant("Red");
        traffic_light.push_variant("Yellow");
        traffic_light.push_variant("Green");

        data_model.push_enum("b.prisma".to_string(), traffic_light);

        let mut cat = Enum::new("Cat");
        cat.push_variant("Asleep");
        cat.push_variant("Hungry");

        data_model.push_enum("c.prisma".to_string(), cat);

        let mut view = View::new("Meow");
        let mut field = Field::new("id", "Int");
        field.id(IdFieldDefinition::default());

        view.push_field(field);

        data_model.push_view("d.prisma".to_string(), view);

        let expected = expect![[r#"
            // file: a.prisma
            type Address {
              street String
            }

            model User {
              id Int @id @default(autoincrement())
            }
            ------
            // file: b.prisma
            enum TrafficLight {
              Red
              Yellow
              Green
            }
            ------
            // file: c.prisma
            enum Cat {
              Asleep
              Hungry
            }
            ------
            // file: d.prisma
            view Meow {
              id Int @id
            }
        "#]];
        let rendered = pretty_render(data_model);

        expected.assert_eq(&rendered);
    }
}
