mod relation;

pub use relation::Relation;

use super::{attributes::BlockAttribute, field::Field, IdDefinition, IndexDefinition};
use crate::value::{Constant, Documentation, Function};
use std::{borrow::Cow, fmt};

#[derive(Debug, Clone, Copy)]
pub(super) enum Commented {
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
    fields: Vec<Field<'a>>,
    indexes: Vec<IndexDefinition<'a>>,
    namespace: Option<BlockAttribute<'a>>,
}

impl<'a> Model<'a> {
    /// Create a new model declaration.
    ///
    /// ```ignore
    /// model User {
    /// //    ^^^^ name
    /// }
    /// ```
    pub fn new(name: impl Into<Cow<'a, str>>) -> Self {
        let name = Constant::new_no_validate(name.into());

        Self {
            name,
            commented_out: Commented::Off,
            map: None,
            documentation: None,
            ignore: None,
            id: None,
            namespace: None,
            fields: Vec::new(),
            indexes: Vec::new(),
        }
    }

    /// Documentation of the model. If called repeatedly, adds the new docs to the end with a
    /// newline. This method is also responsible for avoiding to add the same comment twice (mainly
    /// in reintrospection).
    ///
    /// ```ignore
    /// /// This is the documentation.
    /// model Foo {
    ///   ....
    /// }
    /// ```
    pub fn documentation(&mut self, new_documentation: impl Into<Cow<'a, str>>) {
        let new_documentation: Cow<'_, str> = new_documentation.into();

        if self
            .documentation
            .as_ref()
            .map(|d| d.0.contains(new_documentation.as_ref()))
            .unwrap_or_default()
        {
            return;
        }

        match self.documentation.as_mut() {
            Some(documentation) => documentation.push(new_documentation),
            None => self.documentation = Some(Documentation(new_documentation)),
        }
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
    pub fn map(&mut self, map: impl Into<Cow<'a, str>>) {
        let mut fun = Function::new("map");
        fun.push_param(map.into());

        self.map = Some(BlockAttribute(fun));
    }

    /// The namespace attribute of the model block
    ///
    /// ```ignore
    /// model Foo {
    ///   @@namespace("public")
    ///   ^^^^^^^^^^^^^^^^^^ this
    /// }
    /// ```
    pub fn namespace(&mut self, namespace: impl Into<Cow<'a, str>>) {
        let mut fun = Function::new("namespace");
        fun.push_param(namespace.into());

        self.namespace = Some(BlockAttribute(fun));
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

    /// Push a new field to the end of the model.
    ///
    /// ```ignore
    /// model Foo {
    ///   id  Int    @id
    ///   foo String
    ///   ^^^^^^^^^^ this
    /// }
    /// ```
    pub fn push_field(&mut self, field: Field<'a>) {
        self.fields.push(field);
    }

    /// Push a new field to the beginning of the model.
    /// Extremely inefficient, prefer `push_field` if you can.
    ///
    /// ```ignore
    /// model Foo {
    ///   id  Int    @id
    ///   ^^^^^^^^^^^^^^ this
    ///   foo String
    /// }
    /// ```
    pub fn insert_field_front(&mut self, field: Field<'a>) {
        self.fields.insert(0, field);
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

impl fmt::Display for Model<'_> {
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

        if let Some(ref namespace) = self.namespace {
            writeln!(f, "{comment}{namespace}")?;
        }

        writeln!(f, "{comment}}}")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::{datamodel::*, value::Function};
    use expect_test::expect;

    #[test]
    fn kitchen_sink() {
        let mut model = Model::new("Country");
        model.map("1Country");
        model.documentation("Do not fear death\nIf you love the trail of streaking fire\nDo not fear death\nIf you desire a speed king to become!");

        let mut field = Field::new("id", "String");

        let mut opts = IdFieldDefinition::default();

        opts.sort_order("Desc");
        opts.length(32);
        opts.clustered(false);

        field.id(opts);
        field.native_type("db", "VarChar", vec![String::from("255")]);
        field.default(DefaultValue::function(Function::new("uuid")));

        model.push_field(field);

        let mut field = Field::new("value", "Bytes");
        field.optional();
        field.documentation("NOPEUSKUNINGAS");
        field.default(DefaultValue::bytes(&[1u8, 2, 3, 4] as &[u8]));
        model.push_field(field);

        let mut field = Field::new("array", "Int");
        field.array();
        field.map("1array");
        field.default(DefaultValue::array(vec![1, 2, 3, 4]));
        model.push_field(field);

        let mut field = Field::new("konig", "King");
        field.unsupported();
        field.ignore();
        model.push_field(field);

        let mut field = Field::new("information", "Int");
        let mut opts = UniqueFieldAttribute::default();

        opts.sort_order("Desc");
        opts.length(32);
        opts.clustered(true);

        field.unique(opts);

        model.push_field(field);

        let mut relation = Relation::new();
        relation.fields(["information"].into_iter().map(ToOwned::to_owned).map(Cow::Owned));
        relation.references(["id"].into_iter().map(ToOwned::to_owned).map(Cow::Owned));
        relation.on_delete("Cascade");
        relation.on_update("Restrict");

        let mut field = Field::new("relfield", "Planet");
        field.relation(relation);

        model.push_field(field);

        let fields = ["foo", "bar"].iter().enumerate().map(|(i, name)| {
            if i == 1 {
                IndexFieldInput {
                    name: Cow::Borrowed(name),
                    sort_order: Some("Asc".into()),
                    length: Some(32),
                    ops: None,
                }
            } else {
                IndexFieldInput {
                    name: Cow::Borrowed(name),
                    sort_order: None,
                    length: None,
                    ops: None,
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

        model.namespace("public");
        model.ignore();

        let expected = expect![[r#"
            /// Do not fear death
            /// If you love the trail of streaking fire
            /// Do not fear death
            /// If you desire a speed king to become!
            model Country {
              id          String              @id(sort: Desc, length: 32, clustered: false) @default(uuid()) @db.VarChar(255)
              /// NOPEUSKUNINGAS
              value       Bytes?              @default("AQIDBA==")
              array       Int[]               @default([1, 2, 3, 4]) @map("1array")
              konig       Unsupported("King") @ignore
              information Int                 @unique(sort: Desc, length: 32, clustered: true)
              relfield    Planet              @relation(fields: [information], references: [id], onDelete: Cascade, onUpdate: Restrict)

              @@id([foo, bar(length: 32, sort: Asc)], name: "primary", map: "PKPK", clustered: false)
              @@unique([foo, bar])
              @@index([foo, bar], type: BTree)
              @@fulltext([foo, bar])
              @@map("1Country")
              @@ignore
              @@namespace("public")
            }
        "#]];

        let rendered = psl::reformat(&model.to_string(), 2).unwrap();
        expected.assert_eq(&rendered);
    }

    #[test]
    fn commented_out() {
        let mut model = Model::new("Country");

        let mut field = Field::new("id", "String");
        field.id(IdFieldDefinition::default());
        field.native_type("db", "VarChar", vec![String::from("255")]);
        field.default(DefaultValue::function(Function::new("uuid")));
        model.push_field(field);

        model.namespace("public");
        model.comment_out();

        let expected = expect![[r#"
            // model Country {
            // id String @id @default(uuid()) @db.VarChar(255)
            // @@namespace("public")
            // }
        "#]];

        let rendered = psl::reformat(&model.to_string(), 2).unwrap();
        expected.assert_eq(&rendered);
    }
}
