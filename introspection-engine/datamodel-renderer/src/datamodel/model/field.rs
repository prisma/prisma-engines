use std::{borrow::Cow, fmt};

use crate::{
    datamodel::{attributes::FieldAttribute, DefaultValue, FieldType},
    value::{Constant, ConstantNameValidationError, Documentation, Function},
};

use super::{index_field_input::IndexFieldOptions, Relation};

/// A field in a model block.
#[derive(Debug)]
pub struct ModelField<'a> {
    name: Constant<Cow<'a, str>>,
    commented_out: bool,
    r#type: FieldType<'a>,
    documentation: Option<Documentation<'a>>,
    updated_at: Option<FieldAttribute<'a>>,
    unique: Option<FieldAttribute<'a>>,
    id: Option<FieldAttribute<'a>>,
    default: Option<DefaultValue<'a>>,
    map: Option<FieldAttribute<'a>>,
    relation: Option<Relation<'a>>,
    native_type: Option<FieldAttribute<'a>>,
    ignore: Option<FieldAttribute<'a>>,
}

impl<'a> ModelField<'a> {
    /// Create a new required model field declaration.
    ///
    /// ```ignore
    /// model User {
    ///   name String
    /// //     ^^^^^^ type_name
    /// //^^^^ name
    /// }
    /// ```
    pub fn new_required(name: &'a str, type_name: &'a str) -> Self {
        Self::new(name, FieldType::required(type_name))
    }

    /// Create a new optional model field declaration.
    ///
    /// ```ignore
    /// model Address {
    ///   street String?
    /// //       ^^^^^^ type_name
    /// //^^^^^^ name
    /// }
    /// ```
    pub fn new_optional(name: &'a str, type_name: &'a str) -> Self {
        Self::new(name, FieldType::optional(type_name))
    }

    /// Create a new array model field declaration.
    ///
    /// ```ignore
    /// model Address {
    ///   street String[]
    /// //       ^^^^^^ type_name
    /// //^^^^^^ name
    /// }
    /// ```
    pub fn new_array(name: &'a str, type_name: &'a str) -> Self {
        Self::new(name, FieldType::array(type_name))
    }

    /// Create a new unsupported model field declaration.
    ///
    /// ```ignore
    /// model Address {
    ///   street Unsupported("foo")
    /// //                    ^^^ type_name
    /// //^^^^^^ name
    /// }
    /// ```
    pub fn new_unsupported(name: &'a str, type_name: &'a str) -> Self {
        Self::new(name, FieldType::unsupported(type_name))
    }

    /// Sets the field map attribute.
    ///
    /// ```ignore
    /// model Address {
    ///   street String @map("Straße")
    ///                       ^^^^^^ value
    /// }
    /// ```
    pub fn map(&mut self, value: &'a str) {
        let mut map = Function::new("map");
        map.push_param(value);

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
    pub fn documentation(&mut self, documentation: &'a str) {
        self.documentation = Some(Documentation(documentation));
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
    pub fn native_type(&mut self, prefix: &'a str, r#type: &'a str, params: Vec<String>) {
        let mut native_type = FieldAttribute::new(Function::new(r#type));

        for param in params {
            native_type.push_param(Constant::new_no_validate(param));
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
    pub fn unique(&mut self, options: IndexFieldOptions<'a>) {
        let mut fun = Function::new("unique");

        if let Some(sort_order) = options.sort_order {
            fun.push_param(("sort", Constant::new_no_validate(sort_order)));
        }

        if let Some(length) = options.length {
            fun.push_param(("length", Constant::new_no_validate(length)));
        }

        if let Some(clustered) = options.clustered {
            fun.push_param(("clustered", Constant::new_no_validate(clustered)));
        }

        self.unique = Some(FieldAttribute::new(fun));
    }

    /// Marks the field to be the id of the model.
    ///
    /// ```ignore
    /// model Address {
    ///   street String @id
    /// //              ^^^ this
    /// }
    /// ```
    pub fn id(&mut self, options: IndexFieldOptions<'a>) {
        let mut fun = Function::new("id");

        if let Some(sort_order) = options.sort_order {
            fun.push_param(("sort", Constant::new_no_validate(sort_order)));
        }

        if let Some(length) = options.length {
            fun.push_param(("length", Constant::new_no_validate(length)));
        }

        if let Some(clustered) = options.clustered {
            fun.push_param(("clustered", Constant::new_no_validate(clustered)));
        }

        self.unique = Some(FieldAttribute::new(fun));
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

    fn new(name: &'a str, r#type: FieldType<'a>) -> Self {
        let (name, map, commented_out) = match Constant::new(name) {
            Ok(name) => (name, None, false),
            Err(ConstantNameValidationError::WasSanitized { sanitized }) => {
                let mut map = Function::new("map");
                map.push_param(name);

                let map = FieldAttribute::new(map);

                (sanitized, Some(map), false)
            }
            Err(ConstantNameValidationError::SanitizedEmpty) => {
                let mut map = Function::new("map");
                map.push_param(name);

                let map = FieldAttribute::new(map);

                (Constant::new_no_validate(Cow::Borrowed(name)), Some(map), true)
            }
            Err(ConstantNameValidationError::OriginalEmpty) => {
                todo!("If I left this for PR review, Tom... Remind me to consider something else than a panic.")
            }
        };

        Self {
            name,
            commented_out,
            r#type,
            map,
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
}

impl<'a> fmt::Display for ModelField<'a> {
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
