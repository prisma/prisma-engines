#![deny(missing_docs)]

use crate::{ast, types, ParserDatabase, ScalarFieldType};

/// A composite type, introduced with the `type` keyword in the schema.
///
/// Example:
///
/// ```prisma
/// type Address {
///     name String
///     streetName String
///     streetNumber Int
///     city String
///     zipCode Int
///     countryCode String
/// }
/// ```
#[derive(Copy, Clone)]
pub struct CompositeTypeWalker<'ast, 'db> {
    pub(super) ctid: ast::CompositeTypeId,
    pub(super) db: &'db ParserDatabase<'ast>,
}

impl<'ast, 'db> PartialEq for CompositeTypeWalker<'ast, 'db> {
    fn eq(&self, other: &Self) -> bool {
        self.ctid == other.ctid
    }
}

impl<'ast, 'db> CompositeTypeWalker<'ast, 'db> {
    /// The ID of the composite type node in the AST.
    pub fn composite_type_id(self) -> ast::CompositeTypeId {
        self.ctid
    }

    /// The composite type node in the AST.
    pub fn ast_composite_type(self) -> &'ast ast::CompositeType {
        &self.db.ast()[self.ctid]
    }

    /// The name of the composite type in the schema.
    pub fn name(self) -> &'ast str {
        &self.db.ast[self.ctid].name.name
    }

    /// Iterator over all the fields of the composite type.
    pub fn fields(self) -> impl Iterator<Item = CompositeTypeFieldWalker<'ast, 'db>> {
        let db = self.db;
        db.types
            .composite_type_fields
            .range((self.ctid, ast::FieldId::ZERO)..(self.ctid, ast::FieldId::MAX))
            .map(move |((ctid, field_id), field)| CompositeTypeFieldWalker {
                ctid: *ctid,
                field_id: *field_id,
                field,
                db,
            })
    }
}

/// A field in a composite type.
#[derive(Clone, Copy)]
pub struct CompositeTypeFieldWalker<'ast, 'db> {
    ctid: ast::CompositeTypeId,
    field_id: ast::FieldId,
    field: &'db types::CompositeTypeField<'ast>,
    db: &'db ParserDatabase<'ast>,
}

impl<'ast, 'db> CompositeTypeFieldWalker<'ast, 'db> {
    /// The AST node for the field.
    pub fn ast_field(self) -> &'ast ast::Field {
        &self.db.ast[self.ctid][self.field_id]
    }

    /// The composite type containing the field.
    pub fn composite_type(self) -> CompositeTypeWalker<'ast, 'db> {
        CompositeTypeWalker {
            ctid: self.ctid,
            db: self.db,
        }
    }

    /// The optional documentation string of the field.
    pub fn documentation(&self) -> Option<&str> {
        self.ast_field().documentation.as_ref().map(|c| c.text.as_str())
    }

    /// The name contained in the `@map()` attribute of the field, if any.
    pub fn mapped_name(self) -> Option<&'ast str> {
        self.field.mapped_name
    }

    /// The name of the field.
    pub fn name(self) -> &'ast str {
        &self.ast_field().name.name
    }

    /// Is the field required, optional or a list?
    pub fn arity(self) -> ast::FieldArity {
        self.ast_field().arity
    }

    /// The type of the field, e.g. `String` in `streetName String?`.
    pub fn r#type(self) -> &'db ScalarFieldType {
        &self.field.r#type
    }
}
