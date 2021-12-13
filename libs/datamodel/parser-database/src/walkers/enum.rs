use crate::{ast, walkers::Walker};

/// An `enum` declaration in the schema.
pub type EnumWalker<'ast, 'db> = Walker<'ast, 'db, ast::EnumId>;
/// One value in an `enum` declaration in the schema.
pub type EnumValueWalker<'ast, 'db> = Walker<'ast, 'db, (ast::EnumId, usize)>;

impl<'ast, 'db> EnumWalker<'ast, 'db> {
    /// The name of the enum.
    pub fn name(self) -> &'ast str {
        &self.ast_enum().name.name
    }

    /// The AST node.
    pub fn ast_enum(self) -> &'ast ast::Enum {
        &self.db.ast()[self.id]
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
    pub fn mapped_name(self) -> Option<&'ast str> {
        self.db.types.enum_attributes[&self.id].mapped_name
    }

    /// The values of the enum.
    pub fn values(self) -> impl Iterator<Item = EnumValueWalker<'ast, 'db>> {
        (0..self.ast_enum().values.len()).map(move |idx| Walker {
            db: self.db,
            id: (self.id, idx),
        })
    }
}

impl<'ast, 'db> EnumValueWalker<'ast, 'db> {
    fn r#enum(self) -> EnumWalker<'ast, 'db> {
        Walker {
            db: self.db,
            id: self.id.0,
        }
    }

    /// The enum documentation
    pub fn documentation(self) -> Option<&'ast str> {
        self.r#enum().ast_enum().values[self.id.1]
            .documentation
            .as_ref()
            .map(|doc| doc.text.as_str())
    }

    /// The name of the value.
    pub fn name(self) -> &'ast str {
        &self.r#enum().ast_enum().values[self.id.1].name.name
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
    pub fn mapped_name(self) -> Option<&'ast str> {
        self.db.types.enum_attributes[&self.id.0]
            .mapped_values
            .get(&(self.id.1 as u32))
            .cloned()
    }
}
