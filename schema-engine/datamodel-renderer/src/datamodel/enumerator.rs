use super::attributes::{BlockAttribute, FieldAttribute};
use crate::value::{Constant, Documentation, Function};
use std::{borrow::Cow, fmt};

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
    pub fn map(&mut self, value: impl Into<Cow<'a, str>>) {
        let mut map = Function::new("map");
        map.push_param(value.into());
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
    pub fn comment_out(&mut self) {
        self.comment_out = true;
    }

    /// Documentation of a variant.
    ///
    /// ```ignore
    /// enum Foo {
    ///   /// This is the documentation.
    ///   Bar
    /// }
    /// ```
    pub fn documentation(&mut self, documentation: impl Into<Cow<'a, str>>) {
        self.documentation = Some(Documentation(documentation.into()));
    }
}

impl<'a> From<&'a str> for EnumVariant<'a> {
    fn from(variant: &'a str) -> Self {
        Self::new(Cow::Borrowed(variant))
    }
}

impl fmt::Display for EnumVariant<'_> {
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
    namespace: Option<BlockAttribute<'a>>,
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
    pub fn new(name: impl Into<Cow<'a, str>>) -> Self {
        Self {
            name: Constant::new_no_validate(name.into()),
            documentation: None,
            variants: Vec::new(),
            map: None,
            namespace: None,
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
    pub fn documentation(&mut self, documentation: impl Into<Cow<'a, str>>) {
        self.documentation = Some(Documentation(documentation.into()));
    }

    /// The namespace attribute of the enum block
    ///
    /// ```ignore
    /// enum Foo {
    ///   Bar
    ///
    ///   @@namespace("public")
    ///             ^^^^^^ this
    /// }
    /// ```
    pub fn namespace(&mut self, namespace: impl Into<Cow<'a, str>>) {
        let mut fun = Function::new("namespace");
        fun.push_param(namespace.into());

        self.namespace = Some(BlockAttribute(fun));
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
    pub fn map(&mut self, mapped_name: impl Into<Cow<'a, str>>) {
        let mut fun = Function::new("map");
        fun.push_param(mapped_name.into());

        self.map = Some(BlockAttribute(fun));
    }
}

impl fmt::Display for Enum<'_> {
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

        if let Some(namespace) = &self.namespace {
            writeln!(f, "{namespace}")?;
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
        let mut r#enum = Enum::new("TrafficLight");
        r#enum.map("1TrafficLight");
        r#enum.documentation("Cat's foot, iron claw\nNeuro-surgeons scream for more\nAt paranoia's poison door...");

        r#enum.push_variant("Red");
        {
            let mut green = EnumVariant::new("Green".into());
            green.map("1Green");
            r#enum.push_variant(green);
        }

        let mut variant = EnumVariant::new("Blue".into());
        variant.map("Yellow");
        variant.documentation("Twenty-first century schizoid man!");

        r#enum.push_variant(variant);

        let mut variant = EnumVariant::new("Invalid".into());
        variant.comment_out();
        r#enum.push_variant(variant);
        r#enum.push_variant("Black");

        r#enum.namespace("city_planning");

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
              @@namespace("city_planning")
            }
        "#]];

        let rendered = psl::reformat(&format!("{enum}"), 2).unwrap();
        expected.assert_eq(&rendered);
    }
}
