mod field;
mod id;
mod index;
mod index_field_input;
mod relation;

pub use field::ModelField;
pub use id::{IdDefinition, IdFieldDefinition};
pub use index::{IndexDefinition, IndexOps};
pub use index_field_input::{IndexFieldInput, IndexFieldOptions};
use psl::dml;
pub use relation::Relation;

use crate::value::{Constant, Documentation, Function};
use std::{borrow::Cow, collections::HashMap, fmt};

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

    /// Documentation of the model. If called repeteadly,
    /// adds the new docs to the end with a newline.
    ///
    /// ```ignore
    /// /// This is the documentation.
    /// model Foo {
    ///   ....
    /// }
    /// ```
    pub fn documentation(&mut self, documentation: impl Into<Cow<'a, str>>) {
        match self.documentation.as_mut() {
            Some(docs) => docs.push(documentation),
            None => self.documentation = Some(Documentation(documentation.into())),
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

    /// The schema attribute of the model block
    ///
    /// ```ignore
    /// model Foo {
    ///   @@schema("public")
    ///   ^^^^^^^^^^^^^^^^^^ this
    /// }
    /// ```
    pub fn schema(&mut self, schema: impl Into<Cow<'a, str>>) {
        let mut fun = Function::new("schema");
        fun.push_param(schema.into());

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

    /// Generate a model rendering from the deprecated DML structure.
    ///
    /// Remove when destroying the DML.
    pub fn from_dml(datasource: &'a psl::Datasource, dml_model: &dml::Model) -> Self {
        let mut model = Model::new(dml_model.name.clone());

        if let Some(docs) = &dml_model.documentation {
            model.documentation(docs.clone());
        }

        if let Some(map) = &dml_model.database_name {
            model.map(map.clone());
        }

        if let Some(ref schema) = dml_model.schema {
            model.schema(schema.clone());
        }

        if dml_model.is_commented_out {
            model.comment_out();
        }

        if dml_model.is_ignored {
            model.ignore();
        }

        match dml_model.primary_key {
            Some(ref pk) if !dml_model.has_single_id_field() => {
                let fields = pk.fields.iter().map(|field| IndexFieldInput {
                    name: Cow::Owned(field.name.clone()),
                    sort_order: field.sort_order.as_ref().map(|so| so.as_ref().to_owned().into()),
                    length: field.length,
                    ops: None,
                });

                let mut definition: IdDefinition<'static> = IdDefinition::new(fields);

                if let Some(ref name) = pk.name {
                    definition.name(name.clone());
                }

                if let Some(ref map) = &pk.db_name {
                    definition.map(map.clone());
                }

                if let Some(clustered) = pk.clustered {
                    definition.clustered(clustered);
                }

                model.id(definition);
            }
            _ => (),
        }

        // weep
        let uniques: HashMap<&str, IndexFieldOptions<'static>> = dml_model
            .indices
            .iter()
            .rev() // replicate existing behaviour on duplicate unique constraints
            .filter(|ix| ix.is_unique())
            .filter(|ix| ix.defined_on_field)
            .map(|ix| {
                let definition = ix.fields.first().unwrap();
                let mut opts = IndexFieldOptions::default();

                if let Some(clustered) = ix.clustered {
                    opts.clustered(clustered);
                }

                if let Some(ref sort_order) = definition.sort_order {
                    opts.sort_order(sort_order.as_ref().to_owned());
                }

                if let Some(length) = definition.length {
                    opts.length(length);
                }

                if let Some(ref map) = ix.db_name {
                    opts.map(map.clone());
                }

                (definition.from_field(), opts)
            })
            .collect();

        let primary_key = dml_model.primary_key.as_ref().filter(|pk| pk.defined_on_field);

        for dml_field in dml_model.fields.iter() {
            // sob :(
            let id = primary_key.and_then(|pk| {
                let field = pk.fields.first().unwrap();

                if field.name == dml_field.name() {
                    let mut opts = IdFieldDefinition::default();

                    if let Some(clustered) = pk.clustered {
                        opts.clustered(clustered);
                    }

                    if let Some(ref sort_order) = field.sort_order {
                        opts.sort_order(sort_order.as_ref().to_owned());
                    }

                    if let Some(length) = field.length {
                        opts.length(length);
                    }

                    if let Some(ref map) = pk.db_name {
                        opts.map(map.clone());
                    }

                    Some(opts)
                } else {
                    None
                }
            });

            model.push_field(ModelField::from_dml(datasource, dml_model, dml_field, &uniques, id));
        }

        for dml_index in dml_model.indices.iter() {
            if dml_index.defined_on_field && dml_index.is_unique() {
                continue;
            }

            // cry
            let fields = dml_index.fields.iter().map(|f| {
                let mut name = String::new();
                let mut name_path = f.path.iter().peekable();

                while let Some((ident, _)) = name_path.next() {
                    name.push_str(ident);

                    if name_path.peek().is_some() {
                        name.push('.');
                    }
                }

                let ops = f.operator_class.as_ref().map(|c| {
                    if c.is_raw() {
                        IndexOps::raw(c.as_ref().to_owned())
                    } else {
                        IndexOps::managed(c.as_ref().to_owned())
                    }
                });

                IndexFieldInput {
                    name: Cow::Owned(name),
                    sort_order: f.sort_order.map(|s| s.as_ref().to_string().into()),
                    length: f.length,
                    ops,
                }
            });

            let mut definition = match dml_index.tpe {
                dml::IndexType::Unique => IndexDefinition::unique(fields),
                dml::IndexType::Normal => IndexDefinition::index(fields),
                dml::IndexType::Fulltext => IndexDefinition::fulltext(fields),
            };

            if let Some(ref name) = dml_index.name {
                definition.name(name.clone());
            }

            if let Some(ref map) = dml_index.db_name {
                definition.map(map.clone());
            }

            if let Some(clustered) = dml_index.clustered {
                definition.clustered(clustered);
            }

            if let Some(ref algo) = dml_index.algorithm {
                definition.index_type(algo.as_ref().to_string());
            }

            model.push_index(definition);
        }

        model
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
    use std::borrow::Cow;

    use crate::{datamodel::*, value::Function};
    use expect_test::expect;

    #[test]
    fn kitchen_sink() {
        let mut model = Model::new("Country");
        model.map("1Country");
        model.documentation("Do not fear death\nIf you love the trail of streaking fire\nDo not fear death\nIf you desire a speed king to become!");

        let mut field = ModelField::new("id", "String");

        let mut opts = IdFieldDefinition::default();

        opts.sort_order("Desc");
        opts.length(32);
        opts.clustered(false);

        field.id(opts);
        field.native_type("db", "VarChar", vec![String::from("255")]);
        field.default(DefaultValue::function(Function::new("uuid")));

        model.push_field(field);

        let mut field = ModelField::new("value", "Bytes");
        field.optional();
        field.documentation("NOPEUSKUNINGAS");
        field.default(DefaultValue::bytes(&[1u8, 2, 3, 4] as &[u8]));
        model.push_field(field);

        let mut field = ModelField::new("array", "Int");
        field.array();
        field.map("1array");
        field.default(DefaultValue::array(vec![1, 2, 3, 4]));
        model.push_field(field);

        let mut field = ModelField::new("konig", "King");
        field.unsupported();
        field.ignore();
        model.push_field(field);

        let mut field = ModelField::new("information", "Int");
        let mut opts = IndexFieldOptions::default();

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

        let mut field = ModelField::new("relfield", "Planet");
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
              @@schema("public")
            }
        "#]];

        let rendered = psl::reformat(&model.to_string(), 2).unwrap();
        expected.assert_eq(&rendered);
    }

    #[test]
    fn commented_out() {
        let mut model = Model::new("Country");

        let mut field = ModelField::new("id", "String");
        field.id(IdFieldDefinition::default());
        field.native_type("db", "VarChar", vec![String::from("255")]);
        field.default(DefaultValue::function(Function::new("uuid")));
        model.push_field(field);

        model.schema("public");
        model.comment_out();

        let expected = expect![[r#"
            // model Country {
            // id String @id @default(uuid()) @db.VarChar(255)
            // @@schema("public")
            // }
        "#]];

        let rendered = psl::reformat(&model.to_string(), 2).unwrap();
        expected.assert_eq(&rendered);
    }
}
