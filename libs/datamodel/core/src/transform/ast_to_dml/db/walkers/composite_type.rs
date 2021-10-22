use crate::{
    ast,
    transform::ast_to_dml::db::{types, ParserDatabase, ScalarFieldType},
};

#[derive(Copy, Clone)]
pub(crate) struct CompositeTypeWalker<'ast, 'db> {
    pub(super) ctid: ast::CompositeTypeId,
    pub(super) db: &'db ParserDatabase<'ast>,
}

impl<'ast, 'db> PartialEq for CompositeTypeWalker<'ast, 'db> {
    fn eq(&self, other: &Self) -> bool {
        self.ctid == other.ctid
    }
}

impl<'ast, 'db> CompositeTypeWalker<'ast, 'db> {
    pub(crate) fn composite_type_id(self) -> ast::CompositeTypeId {
        self.ctid
    }

    pub(crate) fn name(self) -> &'ast str {
        &self.db.ast[self.ctid].name.name
    }

    pub(crate) fn fields(self) -> impl Iterator<Item = CompositeTypeFieldWalker<'ast, 'db>> {
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
pub(crate) struct CompositeTypeFieldWalker<'ast, 'db> {
    ctid: ast::CompositeTypeId,
    field_id: ast::FieldId,
    field: &'db types::CompositeTypeField<'ast>,
    db: &'db ParserDatabase<'ast>,
}

impl<'ast, 'db> CompositeTypeFieldWalker<'ast, 'db> {
    pub(crate) fn ast_field(self) -> &'ast ast::Field {
        &self.db.ast[self.ctid][self.field_id]
    }

    pub(crate) fn composite_type(self) -> CompositeTypeWalker<'ast, 'db> {
        CompositeTypeWalker {
            ctid: self.ctid,
            db: self.db,
        }
    }

    pub(crate) fn documentation(&self) -> Option<&str> {
        self.ast_field().documentation.as_ref().map(|c| c.text.as_str())
    }

    pub(crate) fn mapped_name(self) -> Option<&'ast str> {
        self.field.mapped_name
    }

    pub(crate) fn name(self) -> &'ast str {
        &self.ast_field().name.name
    }

    pub(crate) fn arity(self) -> ast::FieldArity {
        self.ast_field().arity
    }

    pub(crate) fn r#type(self) -> &'db ScalarFieldType {
        &self.field.r#type
    }
}
