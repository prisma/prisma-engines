use psl::dml;

use super::attributes::{BlockAttribute, FieldAttribute};
use crate::value::{Constant, ConstantNameValidationError, Documentation, Function};
use std::{borrow::Cow, fmt};

static ENUM_EMPTY_NAME: &str = "ENUM_EMPTY_NAME";

/// A variant declaration in an enum block.
#[derive(Debug)]
pub struct EnumVariant<'a> {
    name: Cow<'a, str>,
    comment_out: bool,
    map: Option<FieldAttribute<'a>>,
    documentation: Option<Documentation<'a>>,
}

impl<'a> EnumVariant<'a> {
    /// Create a new enum variant to be used in an enum declaration.
    ///
    /// ```ignore
    /// enum Foo {
    ///   Bar
    ///   ^^^ value
    /// }
    /// ```
    pub fn new(name: Cow<'a, str>) -> Self {
        Self {
            name,
            comment_out: false,
            map: None,
            documentation: None,
        }
    }

    /// The map attribute of the variant.
    ///
    /// ```ignore
    /// enum Foo {
    ///   Bar @map("foo")
    ///             ^^^ this
    /// }
    /// ```
    pub fn map(&mut self, value: Option<&'a str>) {
        if let Some(value) = value {
            let mut map = Function::new("map");
            map.push_param(value);
            self.map = Some(FieldAttribute::new(map));
        }
    }

    /// Comments the variant out in the declaration.
    ///
    /// ```ignore
    /// enum Foo {
    ///   // Bar
    ///   ^^ adds this
    /// }
    /// ```
    pub fn comment_out(&mut self, comment_out: bool) {
        self.comment_out = comment_out;
    }

    /// Documentation of a variant.
    ///
    /// ```ignore
    /// enum Foo {
    ///   /// This is the documentation.
    ///   Bar
    /// }
    /// ```
    pub fn documentation(&mut self, documentation: Option<&'a str>) {
        self.documentation = documentation.map(Documentation);
    }

    /// A throwaway function to help generate a rendering from the DML structures.
    ///
    /// Delete when removing DML.
    fn from_dml(dml_variant: &'a dml::EnumValue) -> Self {
        let mut variant = Self::new(Cow::Borrowed(&dml_variant.name));
        variant.comment_out(dml_variant.commented_out);
        variant.map(dml_variant.database_name.as_deref());
        variant
    }
}

impl<'a> From<&'a str> for EnumVariant<'a> {
    fn from(variant: &'a str) -> Self {
        Self::new(Cow::Borrowed(variant))
    }
}

impl<'a> fmt::Display for EnumVariant<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref docs) = self.documentation {
            docs.fmt(f)?;
        }

        if self.comment_out {
            f.write_str("// ")?;
        }

        f.write_str(&self.name)?;

        if let Some(ref map) = self.map {
            f.write_str(" ")?;
            map.fmt(f)?;
        }

        Ok(())
    }
}

/// An enum block in a PSL file.
#[derive(Debug)]
pub struct Enum<'a> {
    name: Constant<Cow<'a, str>>,
    documentation: Option<Documentation<'a>>,
    variants: Vec<EnumVariant<'a>>,
    map: Option<BlockAttribute<'a>>,
    schema: Option<BlockAttribute<'a>>,
}

impl<'a> Enum<'a> {
    /// Create a new enum declaration block. Will not be valid without
    /// adding at least one variant.
    ///
    /// ```ignore
    /// enum TrafficLight {
    /// //   ^^^^^^^^^^^^ name
    /// }
    /// ```
    pub fn new(name: &'a str) -> Self {
        let (name, map) = match Constant::new(name) {
            Ok(name) => (name, None),
            Err(ConstantNameValidationError::WasSanitized { sanitized }) => {
                let mut fun = Function::new("map");
                fun.push_param(name);

                (sanitized, Some(BlockAttribute(fun)))
            }
            Err(ConstantNameValidationError::SanitizedEmpty) => {
                let mut fun = Function::new("map");
                fun.push_param(name);

                (
                    Constant::new_no_validate(Cow::Borrowed(name)),
                    Some(BlockAttribute(fun)),
                )
            }
            Err(ConstantNameValidationError::OriginalEmpty) => {
                let mut fun = Function::new("map");
                fun.push_param(name);

                (
                    Constant::new_no_validate(Cow::Borrowed(ENUM_EMPTY_NAME)),
                    Some(BlockAttribute(fun)),
                )
            }
        };

        Self {
            name,
            documentation: None,
            variants: Vec::new(),
            map,
            schema: None,
        }
    }

    /// The documentation on top of the enum declaration.
    ///
    /// ```ignore
    /// /// This here is the documentation.
    /// enum Foo {
    ///   Bar
    /// }
    /// ```
    pub fn documentation(&mut self, documentation: &'a str) {
        self.documentation = Some(Documentation(documentation));
    }

    /// The schema attribute of the enum block
    ///
    /// ```ignore
    /// enum Foo {
    ///   Bar
    ///
    ///   @@schema("public")
    ///             ^^^^^^ this
    /// }
    /// ```
    pub fn schema(&mut self, schema: &'a str) {
        let mut fun = Function::new("schema");
        fun.push_param(schema);

        self.schema = Some(BlockAttribute(fun));
    }

    /// Add a new variant to the enum declaration. If passing a string
    /// slice, adds a simple enum variant. Additionally an
    /// `EnumVariant` can be constructed and passed for additional
    /// settings.
    ///
    /// ```ignore
    /// enum Foo {
    ///   Bar
    ///   ^^^ this
    /// }
    /// ```
    pub fn push_variant(&mut self, variant: impl Into<EnumVariant<'a>>) {
        self.variants.push(variant.into());
    }

    /// Sets the block level map attribute.
    ///
    /// ```ignore
    /// enum Foo {
    ///   @@map("bar")
    ///          ^^^ this
    /// }
    /// ```
    pub fn map(&mut self, mapped_name: &'a str) {
        let mut fun = Function::new("map");
        fun.push_param(mapped_name);

        self.map = Some(BlockAttribute(fun));
    }

    /// A throwaway function to help generate a rendering from the DML structures.
    ///
    /// Delete when removing DML.
    pub fn from_dml(dml_enum: &'a dml::Enum) -> Self {
        let mut r#enum = Self::new(&dml_enum.name);

        if let Some(ref docs) = dml_enum.documentation {
            r#enum.documentation(docs);
        }

        if let Some(ref schema) = dml_enum.schema {
            r#enum.schema(schema);
        }

        if let Some(ref map) = dml_enum.database_name {
            r#enum.map(map);
        }

        for dml_variant in dml_enum.values.iter() {
            r#enum.push_variant(EnumVariant::from_dml(dml_variant));
        }

        r#enum
    }
}

impl<'a> fmt::Display for Enum<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(doc) = &self.documentation {
            doc.fmt(f)?;
        }

        writeln!(f, "enum {} {{", self.name)?;

        for variant in self.variants.iter() {
            writeln!(f, "{variant}")?;
        }

        if let Some(map) = &self.map {
            writeln!(f, "{map}")?;
        }

        if let Some(schema) = &self.schema {
            writeln!(f, "{schema}")?;
        }

        f.write_str("}\n")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;

    #[test]
    fn kitchen_sink() {
        let mut r#enum = Enum::new("1TrafficLight");
        r#enum.documentation("Cat's foot, iron claw\nNeuro-surgeons scream for more\nAt paranoia's poison door...");

        r#enum.push_variant("Red");
        {
            let mut green = EnumVariant::new("Green".into());
            green.map(Some("1Green"));
            r#enum.push_variant(green);
        }

        let mut variant = EnumVariant::new("Blue".into());
        variant.map("Yellow".into());
        variant.documentation(Some("Twenty-first century schizoid man!"));

        r#enum.push_variant(variant);

        let mut variant = EnumVariant::new("Invalid".into());
        variant.comment_out(true);
        r#enum.push_variant(variant);
        r#enum.push_variant("Black");

        r#enum.schema("city_planning");

        let expected = expect![[r#"
            /// Cat's foot, iron claw
            /// Neuro-surgeons scream for more
            /// At paranoia's poison door...
            enum TrafficLight {
              Red
              Green @map("1Green")
              /// Twenty-first century schizoid man!
              Blue  @map("Yellow")
              // Invalid
              Black

              @@map("1TrafficLight")
              @@schema("city_planning")
            }
        "#]];

        let rendered = psl::reformat(&format!("{enum}"), 2).unwrap();
        expected.assert_eq(&rendered);
    }
}
