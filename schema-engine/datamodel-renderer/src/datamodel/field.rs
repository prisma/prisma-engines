use crate::{
    datamodel::{
        DefaultValue, FieldType, IdFieldDefinition, Relation, attributes::FieldAttribute, index::UniqueFieldAttribute,
    },
    value::{Constant, Documentation, Function},
};
use std::{borrow::Cow, fmt};

/// A field in a model block.
#[derive(Debug)]
pub struct Field<'a> {
    name: Constant<Cow<'a, str>>,
    commented_out: bool,
    r#type: FieldType<'a>,
    documentation: Option<Documentation<'a>>,
    updated_at: Option<FieldAttribute<'a>>,
    unique: Option<UniqueFieldAttribute<'a>>,
    id: Option<IdFieldDefinition<'a>>,
    default: Option<DefaultValue<'a>>,
    map: Option<FieldAttribute<'a>>,
    relation: Option<Relation<'a>>,
    native_type: Option<FieldAttribute<'a>>,
    ignore: Option<FieldAttribute<'a>>,
}

impl<'a> Field<'a> {
    /// Create a new required model field declaration.
    ///
    /// ```ignore
    /// model User {
    ///   name String
    /// //     ^^^^^^ type_name
    /// //^^^^ name
    /// }
    /// ```
    pub fn new(name: impl Into<Cow<'a, str>>, type_name: impl Into<Cow<'a, str>>) -> Self {
        let name = Constant::new_no_validate(name.into());

        Self {
            name,
            commented_out: false,
            r#type: FieldType::required(type_name),
            map: None,
            documentation: None,
            updated_at: None,
            unique: None,
            id: None,
            default: None,
            relation: None,
            native_type: None,
            ignore: None,
        }
    }

    /// Sets the field as optional.
    ///
    /// ```ignore
    /// model Address {
    ///   street String?
    /// //             ^ this
    /// }
    /// ```
    pub fn optional(&mut self) {
        self.r#type.into_optional();
    }

    /// Sets the field to be an array.
    ///
    /// ```ignore
    /// model Address {
    ///   street String[]
    /// //             ^^ this
    /// }
    /// ```
    pub fn array(&mut self) {
        self.r#type.into_array();
    }

    /// Sets the field to be unsupported.
    ///
    /// ```ignore
    /// model Address {
    ///   street Unsupported("foo")
    /// //       ^^^^^^^^^^^^^^^^^^ this
    /// }
    /// ```
    pub fn unsupported(&mut self) {
        self.r#type.into_unsupported();
    }

    /// Sets the field map attribute.
    ///
    /// ```ignore
    /// model Address {
    ///   street String @map("Straße")
    ///                       ^^^^^^ value
    /// }
    /// ```
    pub fn map(&mut self, value: impl Into<Cow<'a, str>>) {
        let mut map = Function::new("map");
        map.push_param(value.into());

        self.map = Some(FieldAttribute::new(map));
    }

    /// Documentation of the field.
    ///
    /// ```ignore
    /// model Foo {
    ///   /// This is the documentation.
    ///   bar Int
    /// }
    /// ```
    pub fn documentation(&mut self, documentation: impl Into<Cow<'a, str>>) {
        match self.documentation.as_mut() {
            Some(docs) => docs.push(documentation.into()),
            None => self.documentation = Some(Documentation(documentation.into())),
        }
    }

    /// Sets the field default attribute.
    ///
    /// ```ignore
    /// model Address {
    ///   street String @default("Prenzlauer Allee")
    ///                           ^^^^^^^^^^^^^^^^ value
    /// }
    /// ```
    pub fn default(&mut self, value: DefaultValue<'a>) {
        self.default = Some(value);
    }

    /// Sets the native type of the field.
    ///
    /// ```ignore
    /// model Address {
    ///   street String @db.VarChar(255)
    /// //                          ^^^ param
    /// //                  ^^^^^^^ type_name
    /// //               ^^ prefix
    /// }
    /// ```
    ///
    /// TODO: `params` as `&[&str]` when we get rid of the DML.
    pub fn native_type(
        &mut self,
        prefix: impl Into<Cow<'a, str>>,
        r#type: impl Into<Cow<'a, str>>,
        params: impl IntoIterator<Item = impl Into<String>>,
    ) {
        let mut native_type = FieldAttribute::new(Function::new(r#type));

        for param in params {
            native_type.push_param(Constant::new_no_validate(param.into()));
        }

        native_type.prefix(prefix);

        self.native_type = Some(native_type);
    }

    /// Marks the field to hold the update timestamp.
    ///
    /// ```ignore
    /// model Address {
    ///   street String @updatedAt
    /// //              ^^^^^^^^^^ adds this
    /// }
    /// ```
    pub fn updated_at(&mut self) {
        self.updated_at = Some(FieldAttribute::new(Function::new("updatedAt")));
    }

    /// Marks the field to hold a unique constraint.
    ///
    /// ```ignore
    /// model Address {
    ///   street String @unique(sort: Asc, length: 11)
    /// //              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ this
    /// }
    /// ```
    pub fn unique(&mut self, options: UniqueFieldAttribute<'a>) {
        self.unique = Some(options);
    }

    /// Marks the field to be the id of the model.
    ///
    /// ```ignore
    /// model Address {
    ///   street String @id
    /// //              ^^^ this
    /// }
    /// ```
    pub fn id(&mut self, definition: IdFieldDefinition<'a>) {
        self.id = Some(definition);
    }

    /// Set the field to be a relation.
    ///
    /// ```ignore
    /// model Address {
    ///   street Street @relation(...)
    /// //              ^^^^^^^^^^^^^^ this
    /// }
    /// ```
    pub fn relation(&mut self, relation: Relation<'a>) {
        self.relation = Some(relation);
    }

    /// Ignores the field.
    ///
    /// ```ignore
    /// model Address {
    ///   street Street @ignore
    /// //              ^^^^^^^ this
    /// }
    /// ```
    pub fn ignore(&mut self) {
        self.ignore = Some(FieldAttribute::new(Function::new("ignore")));
    }

    /// Comments the field out.
    pub fn commented_out(&mut self) {
        self.commented_out = true;
    }
}

impl fmt::Display for Field<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref docs) = self.documentation {
            docs.fmt(f)?;
        }

        if self.commented_out {
            f.write_str("// ")?;
        }

        write!(f, "{} {}", self.name, self.r#type)?;

        if let Some(ref updated_at) = self.updated_at {
            write!(f, " {updated_at}")?;
        }

        if let Some(ref unique) = self.unique {
            write!(f, " {unique}")?;
        }

        if let Some(ref id) = self.id {
            write!(f, " {id}")?;
        }

        if let Some(ref def) = self.default {
            write!(f, " {def}")?;
        }

        if let Some(ref map) = self.map {
            write!(f, " {map}")?;
        }

        if let Some(ref relation) = self.relation {
            write!(f, " {relation}")?;
        }

        if let Some(ref nt) = self.native_type {
            write!(f, " {nt}")?;
        }

        if let Some(ref ignore) = self.ignore {
            write!(f, " {ignore}")?;
        }

        Ok(())
    }
}
