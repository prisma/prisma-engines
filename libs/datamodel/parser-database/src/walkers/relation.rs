mod implicit_many_to_many;
mod inline;
mod two_way_embedded_many_to_many;

pub use implicit_many_to_many::ImplicitManyToManyRelationWalker;
pub use inline::{CompleteInlineRelationWalker, InlineRelationWalker};
pub use two_way_embedded_many_to_many::TwoWayEmbeddedManyToManyRelationWalker;

use crate::{ast, relations::*, walkers::*, ScalarFieldType};

/// A relation that has the minimal amount of information for us to create one. Useful for
/// validation purposes. Holds all possible relation types.
pub type RelationWalker<'db> = Walker<'db, RelationId>;

impl<'db> RelationWalker<'db> {
    /// Converts the walker to either an implicit many to many, or a inline relation walker
    /// gathering 1:1 and 1:n relations.
    pub fn refine(self) -> RefinedRelationWalker<'db> {
        if self.get().is_implicit_many_to_many() {
            RefinedRelationWalker::ImplicitManyToMany(ImplicitManyToManyRelationWalker(self))
        } else if self.get().is_two_way_embedded_many_to_many() {
            RefinedRelationWalker::TwoWayEmbeddedManyToMany(TwoWayEmbeddedManyToManyRelationWalker(self))
        } else {
            RefinedRelationWalker::Inline(InlineRelationWalker(self))
        }
    }

    pub(crate) fn has_field(self, model_id: ast::ModelId, field_id: ast::FieldId) -> bool {
        self.get().has_field(model_id, field_id)
    }

    /// The relation attributes parsed from the AST.
    fn get(self) -> &'db Relation {
        &self.db.relations[self.id]
    }
}

/// Splits the relation to different types.
pub enum RefinedRelationWalker<'db> {
    /// 1:1 and 1:n relations, where one side defines the relation arguments.
    Inline(InlineRelationWalker<'db>),
    /// Implicit m:n relation. The arguments are inferred by Prisma.
    ImplicitManyToMany(ImplicitManyToManyRelationWalker<'db>),
    /// Embedded 2-way m:n relation.
    TwoWayEmbeddedManyToMany(TwoWayEmbeddedManyToManyRelationWalker<'db>),
}

impl<'db> RefinedRelationWalker<'db> {
    /// Try interpreting this relation as an inline (1:n or 1:1 — without join table) relation.
    pub fn as_inline(&self) -> Option<InlineRelationWalker<'db>> {
        match self {
            RefinedRelationWalker::Inline(inline) => Some(*inline),
            _ => None,
        }
    }

    /// Try interpreting this relation as an implicit many-to-many relation.
    pub fn as_many_to_many(&self) -> Option<ImplicitManyToManyRelationWalker<'db>> {
        match self {
            RefinedRelationWalker::ImplicitManyToMany(m2m) => Some(*m2m),
            _ => None,
        }
    }
}

/// A scalar inferred by loose/magic reformatting.
#[allow(missing_docs)]
pub struct InferredField<'db> {
    pub name: String,
    pub arity: ast::FieldArity,
    pub tpe: ScalarFieldType,
    pub blueprint: ScalarFieldWalker<'db>,
}

/// The scalar fields on the concrete side of the relation.
pub enum ReferencingFields<'db> {
    /// Existing scalar fields
    Concrete(Box<dyn ExactSizeIterator<Item = ScalarFieldWalker<'db>> + 'db>),
    /// Inferred scalar fields
    Inferred(Vec<InferredField<'db>>),
    /// Error
    NA,
}

fn pascal_case(input: &str) -> String {
    let mut c = input.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn camel_case(input: &str) -> String {
    let mut c = input.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
    }
}
