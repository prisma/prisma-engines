use crate::datamodel::attributes::FieldAttribute;
use crate::datamodel::FieldType;
use crate::{
    datamodel::DefaultValue,
    value::{Constant, Documentation, Function},
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
    pub fn new_required(name: impl Into<Cow<'a, str>>, type_name: impl Into<Cow<'a, str>>) -> Self {
        Self::new(name.into(), FieldType::required(type_name))
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
    pub fn new_optional(name: impl Into<Cow<'a, str>>, type_name: impl Into<Cow<'a, str>>) -> Self {
        Self::new(name.into(), FieldType::optional(type_name))
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
    pub fn new_array(name: impl Into<Cow<'a, str>>, type_name: impl Into<Cow<'a, str>>) -> Self {
        Self::new(name.into(), FieldType::array(type_name))
    }

    /// Create a new required unsupported composite field declaration.
    ///
    /// ```ignore
    /// type Address {
    ///   street Unsupported("foo")
    /// //                    ^^^ type_name
    /// //^^^^^^ name
    /// }
    /// ```
    pub fn new_required_unsupported(name: impl Into<Cow<'a, str>>, type_name: impl Into<Cow<'a, str>>) -> Self {
        Self::new(name.into(), FieldType::required_unsupported(type_name.into()))
    }

    /// Create a new optional unsupported composite field declaration.
    ///
    /// ```ignore
    /// type Address {
    ///   street Unsupported("foo")?
    /// //                    ^^^ type_name
    /// //^^^^^^ name
    /// }
    /// ```
    pub fn new_optional_unsupported(name: impl Into<Cow<'a, str>>, type_name: impl Into<Cow<'a, str>>) -> Self {
        Self::new(name.into(), FieldType::optional_unsupported(type_name))
    }

    /// Create a new array unsupported composite field declaration.
    ///
    /// ```ignore
    /// type Address {
    ///   street Unsupported("foo")[]
    /// //                    ^^^ type_name
    /// //^^^^^^ name
    /// }
    /// ```
    pub fn new_array_unsupported(name: impl Into<Cow<'a, str>>, type_name: impl Into<Cow<'a, str>>) -> Self {
        Self::new(name.into(), FieldType::array_unsupported(type_name))
    }

    /// Sets the field map attribute.
    ///
    /// ```ignore
    /// type Address {
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
    /// type Foo {
    ///   /// This is the documentation.
    ///   bar Int
    /// }
    /// ```
    pub fn documentation(&mut self, documentation: impl Into<Cow<'a, str>>) {
        self.documentation = Some(Documentation(documentation.into()));
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
    pub fn native_type(&mut self, prefix: &'a str, r#type: impl Into<Cow<'a, str>>, params: Vec<String>) {
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
    pub fn from_dml(datasource: &'a psl::Datasource, dml_field: &dml::CompositeTypeField) -> Self {
        let (r#type, native_type) = match dml_field.r#type {
            dml::CompositeTypeFieldType::CompositeType(ref ct) => (ct.clone(), None),
            dml::CompositeTypeFieldType::Scalar(ref st, ref nt) => {
                (st.as_ref().to_owned(), nt.as_ref().map(|nt| (nt.name(), nt.args())))
            }
            dml::CompositeTypeFieldType::Enum(ref s) => (s.clone(), None),
            dml::CompositeTypeFieldType::Unsupported(ref s) => (s.clone(), None),
        };
        let field_name = dml_field.name.clone();

        let mut field = match dml_field.arity {
            dml::FieldArity::Required if dml_field.r#type.is_unsupported() => {
                CompositeTypeField::new_required_unsupported(field_name, r#type)
            }
            dml::FieldArity::Optional if dml_field.r#type.is_unsupported() => {
                CompositeTypeField::new_optional_unsupported(field_name, r#type)
            }
            dml::FieldArity::List if dml_field.r#type.is_unsupported() => {
                CompositeTypeField::new_array_unsupported(field_name, r#type)
            }
            dml::FieldArity::Required => CompositeTypeField::new_required(field_name, r#type),
            dml::FieldArity::Optional => CompositeTypeField::new_optional(field_name, r#type),
            dml::FieldArity::List => CompositeTypeField::new_array(field_name, r#type),
        };

        if let Some(ref docs) = dml_field.documentation {
            field.documentation(docs.clone());
        }

        if dml_field.is_commented_out {
            field.commented_out();
        }

        if let Some(ref map) = dml_field.database_name {
            field.map(map.clone());
        }

        if let Some(ref dml_def) = dml_field.default_value {
            field.default(DefaultValue::from_dml(dml_def));
        }

        if let Some((native_type_name, native_type_args)) = native_type {
            field.native_type(&datasource.name, native_type_name.to_owned(), native_type_args)
        }

        field
    }

    fn new(name: Cow<'a, str>, r#type: FieldType<'a>) -> Self {
        let (name, map, commented_out) = (Constant::new_no_validate(name), None, false);

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
