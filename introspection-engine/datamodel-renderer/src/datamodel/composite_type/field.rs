use crate::datamodel::attributes::FieldAttribute;
use crate::datamodel::FieldType;
use crate::{
    datamodel::DefaultValue,
    value::{Constant, ConstantNameValidationError, Documentation, Function},
};
use psl::dml;
use std::{borrow::Cow, fmt};

/// A field in a composite type block.
#[derive(Debug)]
pub struct CompositeTypeField<'a> {
    name: Constant<Cow<'a, str>>,
    r#type: FieldType<'a>,
    documentation: Option<Documentation<'a>>,
    map: Option<FieldAttribute<'a>>,
    default: Option<DefaultValue<'a>>,
    native_type: Option<FieldAttribute<'a>>,
    commented_out: bool,
}

impl<'a> CompositeTypeField<'a> {
    /// Create a new required composite field declaration.
    ///
    /// ```ignore
    /// type Address {
    ///   street String
    /// //       ^^^^^^ type_name
    /// //^^^^^^ name
    /// }
    /// ```
    pub fn new_required(name: &'a str, type_name: &'a str) -> Self {
        Self::new(name, FieldType::required(type_name))
    }

    /// Create a new optional composite field declaration.
    ///
    /// ```ignore
    /// type Address {
    ///   street String?
    /// //       ^^^^^^ type_name
    /// //^^^^^^ name
    /// }
    /// ```
    pub fn new_optional(name: &'a str, type_name: &'a str) -> Self {
        Self::new(name, FieldType::optional(type_name))
    }

    /// Create a new array composite field declaration.
    ///
    /// ```ignore
    /// type Address {
    ///   street String[]
    /// //       ^^^^^^ type_name
    /// //^^^^^^ name
    /// }
    /// ```
    pub fn new_array(name: &'a str, type_name: &'a str) -> Self {
        Self::new(name, FieldType::array(type_name))
    }

    /// Create a new unsupported composite field declaration.
    ///
    /// ```ignore
    /// type Address {
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
    /// type Address {
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
    /// type Foo {
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
    /// type Address {
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
    /// type Address {
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

    /// Comments the field out.
    pub fn commented_out(&mut self) {
        self.commented_out = true;
    }

    /// Generate a composite type rendering from the deprecated DML structure.
    ///
    /// Remove when destroying the DML.
    pub fn from_dml(datasource: &'a psl::Datasource, dml_field: &'a dml::CompositeTypeField) -> Self {
        let (r#type, native_type) = match dml_field.r#type {
            dml::CompositeTypeFieldType::CompositeType(ref ct) => (ct.as_str(), None),
            dml::CompositeTypeFieldType::Scalar(ref st, ref nt) => {
                (st.as_ref(), nt.as_ref().map(|nt| (nt.name(), nt.args())))
            }
            dml::CompositeTypeFieldType::Enum(ref s) => (s.as_str(), None),
            dml::CompositeTypeFieldType::Unsupported(ref s) => (s.as_str(), None),
        };

        let mut field = match dml_field.arity {
            _ if dml_field.r#type.is_unsupported() => CompositeTypeField::new_unsupported(&dml_field.name, r#type),
            dml::FieldArity::Required => CompositeTypeField::new_required(&dml_field.name, r#type),
            dml::FieldArity::Optional => CompositeTypeField::new_optional(&dml_field.name, r#type),
            dml::FieldArity::List => CompositeTypeField::new_array(&dml_field.name, r#type),
        };

        if let Some(ref docs) = dml_field.documentation {
            field.documentation(docs);
        }

        if dml_field.is_commented_out {
            field.commented_out();
        }

        if let Some(ref map) = dml_field.database_name {
            field.map(map);
        }

        if let Some(ref dml_def) = dml_field.default_value {
            field.default(DefaultValue::from_dml(dml_def));
        }

        if let Some((native_type_name, native_type_args)) = native_type {
            field.native_type(&datasource.name, native_type_name, native_type_args)
        }

        field
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
                unreachable!("The name is a mixture of a collection and field names. It should never be empty");
            }
        };

        Self {
            name,
            r#type,
            map,
            default: None,
            native_type: None,
            documentation: None,
            commented_out,
        }
    }
}

impl<'a> fmt::Display for CompositeTypeField<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(documentation) = &self.documentation {
            documentation.fmt(f)?;
        }

        if self.commented_out {
            f.write_str("// ")?;
        }

        self.name.fmt(f)?;
        f.write_str(" ")?;
        self.r#type.fmt(f)?;

        if let Some(ref map) = self.map {
            f.write_str(" ")?;
            map.fmt(f)?;
        }

        if let Some(ref def) = self.default {
            f.write_str(" ")?;
            def.fmt(f)?;
        }

        if let Some(ref nt) = self.native_type {
            f.write_str(" ")?;
            nt.fmt(f)?;
        }

        Ok(())
    }
}
