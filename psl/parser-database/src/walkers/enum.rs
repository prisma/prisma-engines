use crate::{ast, ast::WithDocumentation, types, walkers::Walker};

/// An `enum` declaration in the schema.
pub type EnumWalker<'db> = Walker<'db, ast::EnumId>;
/// One value in an `enum` declaration in the schema.
pub type EnumValueWalker<'db> = Walker<'db, (ast::EnumId, usize)>;

impl<'db> EnumWalker<'db> {
    fn attributes(self) -> &'db types::EnumAttributes {
        &self.db.types.enum_attributes[&self.id]
    }

    /// The name of the enum.
    pub fn name(self) -> &'db str {
        &self.ast_enum().name.name
    }

    /// The AST node.
    pub fn ast_enum(self) -> &'db ast::Enum {
        &self.db.ast()[self.id]
    }

    /// The database name of the enum.
    pub fn database_name(self) -> &'db str {
        self.mapped_name().unwrap_or_else(|| self.name())
    }

    /// The mapped name of the enum:
    ///
    /// ```ignore
    /// enum Colour {
    ///     RED
    ///     GREEN
    ///     BLUE
    ///
    ///     @@map("Color")
    ///           ^^^^^^^
    /// }
    /// ```
    pub fn mapped_name(self) -> Option<&'db str> {
        self.attributes().mapped_name.map(|id| &self.db[id])
    }

    /// The values of the enum.
    pub fn values(self) -> impl Iterator<Item = EnumValueWalker<'db>> {
        (0..self.ast_enum().values.len()).map(move |idx| self.db.walk((self.id, idx)))
    }

    /// The name of the schema the enum belongs to.
    ///
    /// ```ignore
    /// @@schema("public")
    ///          ^^^^^^^^
    /// ```
    pub fn schema(self) -> Option<(&'db str, ast::Span)> {
        self.attributes().schema.map(|(id, span)| (&self.db[id], span))
    }
}

impl<'db> EnumValueWalker<'db> {
    fn r#enum(self) -> EnumWalker<'db> {
        self.db.walk(self.id.0)
    }

    /// The enum documentation
    pub fn documentation(self) -> Option<&'db str> {
        self.r#enum().ast_enum().values[self.id.1].documentation()
    }

    /// The name of the value.
    pub fn name(self) -> &'db str {
        &self.r#enum().ast_enum().values[self.id.1].name.name
    }

    /// The database name of the enum.
    pub fn database_name(self) -> &'db str {
        self.mapped_name().unwrap_or_else(|| self.name())
    }

    /// The mapped name of the value:
    ///
    /// ```ignore
    /// enum Colour {
    ///     RED @map("scarlet")
    ///     GREEN @map("salad")
    ///                ^^^^^^^
    ///     BLUE @map("schmurf")
    /// }
    /// ```
    pub fn mapped_name(self) -> Option<&'db str> {
        self.db.types.enum_attributes[&self.id.0]
            .mapped_values
            .get(&(self.id.1 as u32))
            .map(|id| &self.db[*id])
    }
}
