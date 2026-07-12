use std::{borrow::Cow, fmt};

use crate::value::{Constant, Documentation, Function};

use super::{Field, IdDefinition, IndexDefinition, attributes::BlockAttribute, model::Commented};

/// Defines a model block.
#[derive(Debug)]
pub struct View<'a> {
    name: Constant<Cow<'a, str>>,
    documentation: Option<Documentation<'a>>,
    commented_out: Commented,
    ignore: Option<BlockAttribute<'a>>,
    id: Option<IdDefinition<'a>>,
    map: Option<BlockAttribute<'a>>,
    fields: Vec<Field<'a>>,
    indexes: Vec<IndexDefinition<'a>>,
    schema: Option<BlockAttribute<'a>>,
}

impl<'a> View<'a> {
    /// Create a new view declaration.
    ///
    /// ```ignore
    /// view User {
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
            schema: None,
            fields: Vec::new(),
            indexes: Vec::new(),
        }
    }

    /// Documentation of the view. If called repeatedly, adds the new docs to the end with a
    /// newline.
    ///
    /// ```ignore
    /// /// This is the documentation.
    /// view Foo {
    ///   ....
    /// }
    /// ```
    pub fn documentation(&mut self, documentation: impl Into<Cow<'a, str>>) {
        match self.documentation.as_mut() {
            Some(docs) => docs.push(documentation.into()),
            None => self.documentation = Some(Documentation(documentation.into())),
        }
    }

    /// Ignore the view.
    ///
    /// ```ignore
    /// view Foo {
    ///   @@ignore
    ///   ^^^^^^^^ this
    /// }
    /// ```
    pub fn ignore(&mut self) {
        self.ignore = Some(BlockAttribute(Function::new("ignore")));
    }

    /// Add a view-level id definition.
    ///
    /// ```ignore
    /// view Foo {
    ///   @@id([field1, field2(sort: Desc)])
    ///   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ this
    /// }
    /// ```
    pub fn id(&mut self, id: IdDefinition<'a>) {
        self.id = Some(id);
    }

    /// Add a view-level mapping.
    ///
    /// ```ignore
    /// view Foo {
    ///   @@map("1Foo")
    ///   ^^^^^^^^^^^^^ this
    /// }
    /// ```
    pub fn map(&mut self, map: impl Into<Cow<'a, str>>) {
        let mut fun = Function::new("map");
        fun.push_param(map.into());

        self.map = Some(BlockAttribute(fun));
    }

    /// The schema attribute of the view block
    ///
    /// ```ignore
    /// view Foo {
    ///   @@schema("public")
    ///   ^^^^^^^^^^^^^^^^^^ this
    /// }
    /// ```
    pub fn schema(&mut self, schema: impl Into<Cow<'a, str>>) {
        let mut fun = Function::new("schema");
        fun.push_param(schema.into());

        self.schema = Some(BlockAttribute(fun));
    }

    /// Push a new field to the view.
    ///
    /// ```ignore
    /// view Foo {
    ///   id Int @id
    ///   ^^^^^^^^^^ this
    /// }
    /// ```
    pub fn push_field(&mut self, field: Field<'a>) {
        self.fields.push(field);
    }

    /// Push a new index to the view.
    ///
    /// ```ignore
    /// view Foo {
    ///   @@index([field1, field2])
    ///   ^^^^^^^^^^^^^^^^^^^^^^^^^ this
    /// }
    /// ```
    pub fn push_index(&mut self, index: IndexDefinition<'a>) {
        self.indexes.push(index);
    }
}

impl fmt::Display for View<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Prefix everything with this, so if the model is commented out, so
        // is your line.
        let comment = self.commented_out;

        if let Some(ref docs) = self.documentation {
            docs.fmt(f)?;
        }

        writeln!(f, "{comment}view {} {{", self.name)?;

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
    use std::borrow::Cow;

    use crate::{datamodel::*, value::Function};
    use expect_test::expect;

    #[test]
    fn kitchen_sink() {
        let mut model = View::new("Country");
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

        model.schema("public");
        model.ignore();

        let expected = expect![[r#"
            /// Do not fear death
            /// If you love the trail of streaking fire
            /// Do not fear death
            /// If you desire a speed king to become!
            view Country {
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
              @@schema("public")
            }
        "#]];

        let rendered = psl::reformat(&model.to_string(), 2).unwrap();
        expected.assert_eq(&rendered);
    }
}
