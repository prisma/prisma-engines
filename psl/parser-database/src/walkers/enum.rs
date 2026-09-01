use crate::{
    ast::{self, IndentationType, NewlineType, WithDocumentation, WithName},
    types,
    walkers::{Walker, newline},
};

/// An `enum` declaration in the schema.
pub type EnumWalker<'db> = Walker<'db, crate::EnumId>;
/// One value in an `enum` declaration in the schema.
pub type EnumValueWalker<'db> = Walker<'db, (crate::EnumId, ast::EnumValueId)>;

impl<'db> EnumWalker<'db> {
    fn attributes(self) -> &'db types::EnumAttributes {
        &self.db.types.enum_attributes[&self.id]
    }

    /// The name of the enum.
    pub fn name(self) -> &'db str {
        self.ast_enum().name()
    }

    /// The AST node.
    pub fn ast_enum(self) -> &'db ast::Enum {
        &self.db.asts[self.id]
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

    /// Returns the specific value from the model.
    pub fn value(self, value_id: ast::EnumValueId) -> EnumValueWalker<'db> {
        self.walk((self.id, value_id))
    }

    /// The values of the enum.
    pub fn values(self) -> impl ExactSizeIterator<Item = EnumValueWalker<'db>> {
        self.ast_enum()
            .iter_values()
            .map(move |(value_id, _)| self.walk((self.id, value_id)))
    }

    /// How fields are indented in the enum.
    pub fn indentation(self) -> IndentationType {
        IndentationType::default()
    }

    /// What kind of newlines the enum uses.
    pub fn newline(self) -> NewlineType {
        let value = match self.ast_enum().values.last() {
            Some(value) => value,
            None => return NewlineType::default(),
        };

        let src = self.db.source(self.id.0);

        newline(src, value.span)
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
    /// The AST node.
    pub fn ast_value(self) -> &'db ast::EnumValue {
        &self.db.asts[self.id.0][self.id.1]
    }

    /// The enum documentation
    pub fn documentation(self) -> Option<&'db str> {
        self.ast_value().documentation()
    }

    /// The name of the value.
    pub fn name(self) -> &'db str {
        self.ast_value().name()
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
            .get(&(self.id.1))
            .map(|id| &self.db[*id])
    }

    /// True if the enum value is ignored.
    pub fn is_ignored(self) -> bool {
        self.db.types.enum_attributes[&self.id.0]
            .ignored_values
            .contains(&self.id.1)
    }
}
