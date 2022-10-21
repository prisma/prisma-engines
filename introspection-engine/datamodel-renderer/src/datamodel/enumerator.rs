use super::attributes::{BlockAttribute, FieldAttribute};
use crate::value::{Constant, ConstantNameValidationError, Documentation, Function};
use std::fmt;

static ENUM_EMPTY_VALUE: &str = "EMPTY_ENUM_VALUE";
static ENUM_EMPTY_NAME: &str = "ENUM_EMPTY_NAME";

#[derive(Debug)]
enum EnumVariantKind<'a> {
    Valid(Constant<'a>),
    CommentedOut(Constant<'a>),
}

impl<'a> fmt::Display for EnumVariantKind<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnumVariantKind::Valid(s) => s.fmt(f),
            EnumVariantKind::CommentedOut(c) => {
                f.write_str("// ")?;
                c.fmt(f)
            }
        }
    }
}

/// A variant declaration in an enum block.
#[derive(Debug)]
pub struct EnumVariant<'a> {
    kind: EnumVariantKind<'a>,
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
    pub fn new(value: &'a str) -> Self {
        let (kind, map) = match Constant::new(value) {
            Ok(constant) => {
                let kind = EnumVariantKind::Valid(constant);
                (kind, None)
            }
            Err(ConstantNameValidationError::WasSanitized { sanitized, original }) => {
                let mut fun = Function::new("map");
                fun.push_param(original);

                let kind = EnumVariantKind::Valid(sanitized);

                (kind, Some(FieldAttribute::new(fun)))
            }
            Err(ConstantNameValidationError::OriginalEmpty) => {
                let mut fun = Function::new("map");
                fun.push_param(value);

                let kind = EnumVariantKind::Valid(Constant::new_no_validate(ENUM_EMPTY_VALUE));

                (kind, Some(FieldAttribute::new(fun)))
            }
            Err(ConstantNameValidationError::SanitizedEmpty) => {
                let mut fun = Function::new("map");
                fun.push_param(value);

                let kind = EnumVariantKind::CommentedOut(Constant::new_no_validate(value));

                (kind, Some(FieldAttribute::new(fun)))
            }
        };

        Self {
            kind,
            map,
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
    pub fn map(&mut self, value: &'a str) {
        let mut map = Function::new("map");
        map.push_param(value);

        self.map = Some(FieldAttribute::new(map));
    }

    /// Comments the variant out in the declaration.
    ///
    /// ```ignore
    /// enum Foo {
    ///   // Bar
    ///   ^^ adds this
    /// }
    /// ```
    pub fn into_commented_out(mut self) -> Self {
        if let EnumVariantKind::Valid(value) = self.kind {
            self.kind = EnumVariantKind::CommentedOut(value);
        }

        self
    }

    /// Documentation of a variant.
    ///
    /// ```ignore
    /// enum Foo {
    ///   /// This is the documentation.
    ///   Bar
    /// }
    /// ```
    pub fn documentation(&mut self, documentation: &'a str) {
        self.documentation = Some(Documentation(documentation));
    }
}

impl<'a> From<&'a str> for EnumVariant<'a> {
    fn from(variant: &'a str) -> Self {
        Self::new(variant)
    }
}

impl<'a> fmt::Display for EnumVariant<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref docs) = self.documentation {
            docs.fmt(f)?;
        }

        self.kind.fmt(f)?;

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
    name: Constant<'a>,
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
            Err(ConstantNameValidationError::WasSanitized { sanitized, original }) => {
                let mut fun = Function::new("map");
                fun.push_param(original);

                (sanitized, Some(BlockAttribute(fun)))
            }
            Err(ConstantNameValidationError::SanitizedEmpty) => {
                let mut fun = Function::new("map");
                fun.push_param(name);

                (Constant::new_no_validate(name), Some(BlockAttribute(fun)))
            }
            Err(ConstantNameValidationError::OriginalEmpty) => {
                let mut fun = Function::new("map");
                fun.push_param(name);

                (Constant::new_no_validate(ENUM_EMPTY_NAME), Some(BlockAttribute(fun)))
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
        r#enum.push_variant("1Green");

        let mut variant = EnumVariant::new("-Blue");
        variant.map("Yellow");
        variant.documentation("Twenty-first century schizoid man!");

        r#enum.push_variant(variant);

        let variant = EnumVariant::new("Invalid").into_commented_out();
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
