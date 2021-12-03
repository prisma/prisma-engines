use crate::{ast, types, ParserDatabase, ScalarFieldType};

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
    pub fn composite_type_id(self) -> ast::CompositeTypeId {
        self.ctid
    }

    pub fn ast_composite_type(self) -> &'ast ast::CompositeType {
        &self.db.ast()[self.ctid]
    }

    pub fn name(self) -> &'ast str {
        &self.db.ast[self.ctid].name.name
    }

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

#[derive(Clone, Copy)]
pub struct CompositeTypeFieldWalker<'ast, 'db> {
    ctid: ast::CompositeTypeId,
    field_id: ast::FieldId,
    field: &'db types::CompositeTypeField<'ast>,
    db: &'db ParserDatabase<'ast>,
}

impl<'ast, 'db> CompositeTypeFieldWalker<'ast, 'db> {
    pub fn ast_field(self) -> &'ast ast::Field {
        &self.db.ast[self.ctid][self.field_id]
    }

    pub fn composite_type(self) -> CompositeTypeWalker<'ast, 'db> {
        CompositeTypeWalker {
            ctid: self.ctid,
            db: self.db,
        }
    }

    pub fn documentation(&self) -> Option<&str> {
        self.ast_field().documentation.as_ref().map(|c| c.text.as_str())
    }

    pub fn mapped_name(self) -> Option<&'ast str> {
        self.field.mapped_name
    }

    pub fn name(self) -> &'ast str {
        &self.ast_field().name.name
    }

    pub fn arity(self) -> ast::FieldArity {
        self.ast_field().arity
    }

    pub fn r#type(self) -> &'db ScalarFieldType {
        &self.field.r#type
    }
}
