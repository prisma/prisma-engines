mod field;
mod id;
mod index;
mod index_field_input;
mod relation;

pub use field::ModelField;
pub use id::IdDefinition;
pub use index::IndexDefinition;
pub use index_field_input::{IndexFieldInput, IndexFieldOptions};
pub use relation::Relation;

use crate::value::{Constant, ConstantNameValidationError, Documentation, Function};
use std::{borrow::Cow, fmt};

use super::attributes::BlockAttribute;

#[derive(Debug, Clone, Copy)]
enum Commented {
    On,
    Off,
}

impl fmt::Display for Commented {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Commented::On => f.write_str("// "),
            Commented::Off => Ok(()),
        }
    }
}

/// Defines a model block.
#[derive(Debug)]
pub struct Model<'a> {
    name: Constant<Cow<'a, str>>,
    documentation: Option<Documentation<'a>>,
    commented_out: Commented,
    ignore: Option<BlockAttribute<'a>>,
    id: Option<IdDefinition<'a>>,
    map: Option<BlockAttribute<'a>>,
    fields: Vec<ModelField<'a>>,
    indexes: Vec<IndexDefinition<'a>>,
    schema: Option<BlockAttribute<'a>>,
}

impl<'a> Model<'a> {
    /// Create a new model declaration.
    ///
    /// ```ignore
    /// model User {
    /// //    ^^^^ name
    /// }
    /// ```
    pub fn new(name: &'a str) -> Self {
        let (name, map, commented_out) = match Constant::new(name) {
            Ok(name) => (name, None, Commented::Off),
            Err(ConstantNameValidationError::WasSanitized { sanitized }) => {
                let mut map = Function::new("map");
                map.push_param(name);

                let map = BlockAttribute(map);

                (sanitized, Some(map), Commented::Off)
            }
            Err(ConstantNameValidationError::SanitizedEmpty) => {
                let mut map = Function::new("map");
                map.push_param(name);

                let map = BlockAttribute(map);

                (Constant::new_no_validate(Cow::Borrowed(name)), Some(map), Commented::On)
            }
            Err(ConstantNameValidationError::OriginalEmpty) => {
                todo!("If I left this for PR review, Tom... Remind me to consider something else than a panic.")
            }
        };

        Self {
            name,
            commented_out,
            map,
            documentation: None,
            ignore: None,
            id: None,
            schema: None,
            fields: Vec::new(),
            indexes: Vec::new(),
        }
    }

    /// Documentation of the model.
    ///
    /// ```ignore
    /// /// This is the documentation.
    /// model Foo {
    ///   ....
    /// }
    /// ```
    pub fn documentation(&mut self, documentation: &'a str) {
        self.documentation = Some(Documentation(documentation));
    }

    /// Ignore the model.
    ///
    /// ```ignore
    /// model Foo {
    ///   @@ignore
    ///   ^^^^^^^^ this
    /// }
    /// ```
    pub fn ignore(&mut self) {
        self.ignore = Some(BlockAttribute(Function::new("ignore")));
    }

    /// Add a model-level id definition.
    ///
    /// ```ignore
    /// model Foo {
    ///   @@id([field1, field2(sort: Desc)])
    ///   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ this
    /// }
    /// ```
    pub fn id(&mut self, id: IdDefinition<'a>) {
        self.id = Some(id);
    }

    /// Add a model-level mapping.
    ///
    /// ```ignore
    /// model Foo {
    ///   @@map("1Foo")
    ///   ^^^^^^^^^^^^^ this
    /// }
    /// ```
    pub fn map(&mut self, map: &'a str) {
        let mut fun = Function::new("map");
        fun.push_param(map);

        self.map = Some(BlockAttribute(fun));
    }

    /// The schema attribute of the model block
    ///
    /// ```ignore
    /// model Foo {
    ///   @@schema("public")
    ///   ^^^^^^^^^^^^^^^^^^ this
    /// }
    /// ```
    pub fn schema(&mut self, schema: &'a str) {
        let mut fun = Function::new("schema");
        fun.push_param(schema);

        self.schema = Some(BlockAttribute(fun));
    }

    /// Comments the complete model block out.
    ///
    /// ```ignore
    /// // model Foo {
    /// //   id Int @id
    /// // }
    /// ```
    pub fn comment_out(&mut self) {
        self.commented_out = Commented::On
    }

    /// Push a new field to the model.
    ///
    /// ```ignore
    /// model Foo {
    ///   id Int @id
    ///   ^^^^^^^^^^ this
    /// }
    /// ```
    pub fn push_field(&mut self, field: ModelField<'a>) {
        self.fields.push(field);
    }

    /// Push a new index to the model.
    ///
    /// ```ignore
    /// model Foo {
    ///   @@index([field1, field2])
    ///   ^^^^^^^^^^^^^^^^^^^^^^^^^ this
    /// }
    /// ```
    pub fn push_index(&mut self, index: IndexDefinition<'a>) {
        self.indexes.push(index);
    }
}

impl<'a> fmt::Display for Model<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Prefix everything with this, so if the model is commented out, so
        // is your line.
        let comment = self.commented_out;

        if let Some(ref docs) = self.documentation {
            docs.fmt(f)?;
        }

        writeln!(f, "{comment}model {} {{", self.name)?;

        for field in self.fields.iter() {
            writeln!(f, "{comment}{field}")?;
        }

        if let Some(ref id) = self.id {
            writeln!(f, "{comment}{id}")?;
        }

        for index in self.indexes.iter() {
            writeln!(f, "{comment}{index}")?;
        }

        if let Some(ref map) = self.map {
            writeln!(f, "{comment}{map}")?;
        }

        if let Some(ref ignore) = self.ignore {
            writeln!(f, "{comment}{ignore}")?;
        }

        if let Some(ref schema) = self.schema {
            writeln!(f, "{comment}{schema}")?;
        }

        writeln!(f, "{comment}}}")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{datamodel::*, value::Function};
    use expect_test::expect;

    #[test]
    fn kitchen_sink() {
        let mut model = Model::new("1Country");
        model.documentation("Do not fear death\nIf you love the trail of streaking fire\nDo not fear death\nIf you desire a speed king to become!");

        let mut field = ModelField::new_required("id", "String");

        let mut opts = IndexFieldOptions::default();

        opts.sort_order("Desc");
        opts.length(32);
        opts.clustered(false);

        field.id(opts);
        field.native_type("db", "VarChar", vec![String::from("255")]);
        field.default(DefaultValue::function(Function::new("uuid")));

        model.push_field(field);

        let mut field = ModelField::new_optional("value", "Bytes");
        field.documentation("NOPEUSKUNINGAS");
        field.default(DefaultValue::bytes(&[1, 2, 3, 4]));
        model.push_field(field);

        let mut field = ModelField::new_array("1array", "Int");
        field.default(DefaultValue::array(vec![1, 2, 3, 4]));
        model.push_field(field);

        let mut field = ModelField::new_unsupported("konig", "King");
        field.ignore();
        model.push_field(field);

        let mut field = ModelField::new_required("information", "Int");
        field.unique(IndexFieldOptions {
            sort_order: Some("Desc"),
            length: Some(32),
            clustered: Some(true),
        });
        model.push_field(field);

        let fields = ["foo", "bar"].iter().enumerate().map(|(i, name)| {
            if i == 1 {
                IndexFieldInput {
                    name,
                    sort_order: Some("Asc"),
                    length: Some(32),
                }
            } else {
                IndexFieldInput {
                    name,
                    sort_order: None,
                    length: None,
                }
            }
        });

        let mut id = IdDefinition::new(fields);
        id.name("primary");
        id.map("PKPK");
        id.clustered(false);
        model.id(id);

        let unique = IndexDefinition::unique(["foo", "bar"].iter().map(|s| IndexFieldInput::new(*s)));
        model.push_index(unique);

        let mut index = IndexDefinition::index(["foo", "bar"].iter().map(|s| IndexFieldInput::new(*s)));
        index.index_type("BTree");
        model.push_index(index);

        let fulltext = IndexDefinition::fulltext(["foo", "bar"].iter().map(|s| IndexFieldInput::new(*s)));
        model.push_index(fulltext);

        model.schema("public");
        model.ignore();

        let expected = expect![[r#"
            /// Do not fear death
            /// If you love the trail of streaking fire
            /// Do not fear death
            /// If you desire a speed king to become!
            model Country {
              id          String              @id(sort: Desc, length: 32, clustered: false) @default(uuid) @db.VarChar(255)
              /// NOPEUSKUNINGAS
              value       Bytes?              @default("AQIDBA==")
              array       Int[]               @default([1, 2, 3, 4]) @map("1array")
              konig       Unsupported("King") @ignore
              information Int                 @unique(sort: Desc, length: 32, clustered: true)

              @@id([foo, bar(length: 32, sort: Asc)], name: "primary", map: "PKPK", clustered: false)
              @@unique([foo, bar])
              @@index([foo, bar], type: BTree)
              @@fulltext([foo, bar])
              @@map("1Country")
              @@ignore
              @@schema("public")
            }
        "#]];

        let rendered = psl::reformat(&model.to_string(), 2).unwrap();
        expected.assert_eq(&rendered);
    }

    #[test]
    fn commented_out() {
        let mut model = Model::new("Country");

        let mut field = ModelField::new_required("id", "String");
        field.id(IndexFieldOptions::default());
        field.native_type("db", "VarChar", vec![String::from("255")]);
        field.default(DefaultValue::function(Function::new("uuid")));
        model.push_field(field);

        model.schema("public");
        model.comment_out();

        let expected = expect![[r#"
            // model Country {
            // id String @id @default(uuid) @db.VarChar(255)
            // @@schema("public")
            // }
        "#]];

        let rendered = psl::reformat(&model.to_string(), 2).unwrap();
        expected.assert_eq(&rendered);
    }
}
