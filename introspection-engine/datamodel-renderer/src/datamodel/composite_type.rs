use std::fmt;

use crate::{
    datamodel::DefaultValue,
    value::{Constant, ConstantNameValidationError, Documentation, Function},
};

use super::attributes::FieldAttribute;

/// A type block in a PSL file.
pub struct CompositeType<'a> {
    name: Constant<'a>,
    documentation: Option<Documentation<'a>>,
    fields: Vec<CompositeTypeField<'a>>,
}

impl<'a> CompositeType<'a> {
    /// Create a new type declaration block. Will not be valid without
    /// adding at least one field.
    ///
    /// ```ignore
    /// type Address {
    /// //   ^^^^^^^ name
    /// }
    /// ```
    pub fn new(name: &'a str) -> Self {
        let name = Constant::new_no_validate(name);

        Self {
            name,
            documentation: None,
            fields: Vec::new(),
        }
    }

    /// Documentation of the type.
    ///
    /// ```ignore
    /// /// This is the documentation.
    /// type Foo {
    ///   ....
    /// }
    /// ```
    pub fn documentation(&mut self, documentation: &'a str) {
        self.documentation = Some(Documentation(documentation));
    }

    /// Add a new field to the type.
    ///
    /// ```ignore
    /// type Foo {
    ///     bar String
    /// //  ^^^^^^^^^^ this
    /// }
    /// ```
    pub fn push_field(&mut self, field: CompositeTypeField<'a>) {
        self.fields.push(field);
    }
}

impl<'a> fmt::Display for CompositeType<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref docs) = self.documentation {
            docs.fmt(f)?;
        }

        writeln!(f, "type {} {{", self.name)?;

        for field in self.fields.iter() {
            writeln!(f, "{field}")?;
        }

        f.write_str("}\n")?;

        Ok(())
    }
}

/// A type of a field in the datamodel.
pub enum FieldType<'a> {
    /// The field is required, rendered with only the name of the
    /// type. For example: `Int`.
    Required(Constant<'a>),
    /// The field is optional, rendered with a question mark after the
    /// type name. For example: `Int?`.
    Optional(Constant<'a>),
    /// The field is an array, rendered with square brackets after the
    /// type name. For example: `Int[]`.
    Array(Constant<'a>),
}

impl<'a> fmt::Display for FieldType<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldType::Required(ref t) => t.fmt(f),
            FieldType::Optional(ref t) => {
                t.fmt(f)?;
                f.write_str("?")
            }
            FieldType::Array(ref t) => {
                t.fmt(f)?;
                f.write_str("[]")
            }
        }
    }
}

/// A field in a composite type block.
pub struct CompositeTypeField<'a> {
    name: Constant<'a>,
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
        Self::new(name, FieldType::Required(Constant::new_no_validate(type_name)))
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
        Self::new(name, FieldType::Optional(Constant::new_no_validate(type_name)))
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
        Self::new(name, FieldType::Array(Constant::new_no_validate(type_name)))
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
    pub fn native_type(&mut self, prefix: &'a str, r#type: &'a str, params: &[&'a str]) {
        let mut native_type = FieldAttribute::new(Function::new(r#type));

        for param in params {
            native_type.push_param(Constant::new_no_validate(*param));
        }

        native_type.prefix(prefix);

        self.native_type = Some(native_type);
    }

    /// Comments the field out.
    pub fn commented_out(&mut self) {
        self.commented_out = true;
    }

    fn new(name: &'a str, r#type: FieldType<'a>) -> Self {
        let (name, map, commented_out) = match Constant::new(name) {
            Ok(name) => (name, None, false),
            Err(ConstantNameValidationError::WasSanitized { sanitized, original }) => {
                let mut map = Function::new("map");
                map.push_param(original);

                let map = FieldAttribute::new(map);

                (sanitized, Some(map), false)
            }
            Err(ConstantNameValidationError::SanitizedEmpty) => {
                let mut map = Function::new("map");
                map.push_param(name);

                let map = FieldAttribute::new(map);

                (Constant::new_no_validate(name), Some(map), true)
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

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use crate::datamodel::*;

    #[test]
    fn kitchen_sink() {
        let mut composite_type = CompositeType::new("Address");
        composite_type.documentation("...so many tears 🎵");

        let mut field = CompositeTypeField::new_required("Street", "String");
        field.native_type("db", "VarChar", &["255"]);
        field.default(DefaultValue::text("Prenzlauer Allee 193"));
        field.map("Shield");
        composite_type.push_field(field);

        let field = CompositeTypeField::new_required("Number", "Int");
        composite_type.push_field(field);

        let mut field = CompositeTypeField::new_optional("City", "String");
        field.documentation("...soooooooo many tears 🎵");
        composite_type.push_field(field);

        let field = CompositeTypeField::new_array("Other", "String");
        composite_type.push_field(field);

        let field = CompositeTypeField::new_required("1Invalid", "Float");
        composite_type.push_field(field);

        let field = CompositeTypeField::new_required("11111", "Float");
        composite_type.push_field(field);

        let expected = expect![[r#"
            /// ...so many tears 🎵
            type Address {
              Street  String   @default("Prenzlauer Allee 193") @map("Shield") @db.VarChar(255)
              Number  Int
              /// ...soooooooo many tears 🎵
              City    String?
              Other   String[]
              Invalid Float    @map("1Invalid")
              // 11111 Float @map("11111")
            }
        "#]];

        let rendered = psl::reformat(&format!("{composite_type}"), 2).unwrap();
        expected.assert_eq(&rendered);
    }
}
