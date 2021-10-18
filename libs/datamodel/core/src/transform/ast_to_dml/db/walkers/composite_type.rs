use crate::{
    ast,
    transform::ast_to_dml::db::{types, ParserDatabase, ScalarFieldType},
};

#[derive(Copy, Clone)]
pub(crate) struct CompositeTypeWalker<'ast, 'db> {
    pub(super) ctid: ast::CompositeTypeId,
    pub(super) db: &'db ParserDatabase<'ast>,
}

impl<'ast, 'db> CompositeTypeWalker<'ast, 'db> {
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

pub(crate) struct CompositeTypeFieldWalker<'ast, 'db> {
    ctid: ast::CompositeTypeId,
    field_id: ast::FieldId,
    field: &'db types::CompositeTypeField<'ast>,
    db: &'db ParserDatabase<'ast>,
}

impl<'ast, 'db> CompositeTypeFieldWalker<'ast, 'db> {
    pub(crate) fn ast_field(&self) -> &'ast ast::Field {
        &self.db.ast[self.ctid][self.field_id]
    }

    pub(crate) fn mapped_name(&self) -> Option<&'ast str> {
        self.field.mapped_name
    }

    pub(crate) fn name(&self) -> &'ast str {
        &self.ast_field().name.name
    }

    pub(crate) fn arity(&self) -> ast::FieldArity {
        self.ast_field().arity
    }

    pub(crate) fn r#type(&self) -> &'db ScalarFieldType {
        &self.field.r#type
    }
}
