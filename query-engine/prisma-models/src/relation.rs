use psl::{
    datamodel_connector::walker_ext_traits::*,
    parser_database::{walkers, ReferentialAction, RelationId},
};

pub type Relation = crate::Zipper<RelationId>;

impl Relation {
    pub fn name(&self) -> String {
        self.walker().relation_name().to_string()
    }

    /// Returns `true` only if the `Relation` is just a link between two
    /// `RelationField`s.
    pub fn is_inline_relation(&self) -> bool {
        self.walker().refine().as_inline().is_some()
    }

    /// Returns `true` if the `Relation` is a table linking two models.
    pub fn is_relation_table(&self) -> bool {
        !self.is_inline_relation()
    }

    /// A model that relates to itself. For example a `Person` that is a parent
    /// can relate to people that are children.
    pub fn is_self_relation(&self) -> bool {
        self.walker().is_self_relation()
    }

    /// Practically deprecated with Prisma 2.
    pub fn is_many_to_many(&self) -> bool {
        self.walker().relation_fields().all(|f| f.ast_field().arity.is_list())
    }

    pub fn is_one_to_one(&self) -> bool {
        match self.walker().refine() {
            walkers::RefinedRelationWalker::Inline(r) => r.is_one_to_one(),
            _ => false,
        }
    }

    pub fn is_one_to_many(&self) -> bool {
        match self.walker().refine() {
            walkers::RefinedRelationWalker::Inline(r) => !r.is_one_to_one(),
            _ => false,
        }
    }

    /// Retrieves the onDelete policy for this relation.
    pub fn on_delete(&self) -> ReferentialAction {
        let walker = self.walker();
        walker
            .relation_fields()
            .find_map(|rf| rf.explicit_on_delete())
            .unwrap_or_else(|| {
                walker
                    .relation_fields()
                    .next()
                    .unwrap()
                    .default_on_delete_action(self.dm.schema.relation_mode(), self.dm.schema.connector)
            })
    }

    /// Retrieves the onUpdate policy for this relation.
    pub fn on_update(&self) -> ReferentialAction {
        self.walker()
            .relation_fields()
            .find_map(|rf| rf.explicit_on_update())
            .unwrap_or(ReferentialAction::Cascade)
    }
}

impl std::fmt::Debug for Relation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Relation").field(&self.name()).finish()
    }
}
