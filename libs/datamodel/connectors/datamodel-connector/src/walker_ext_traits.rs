use crate::{
    constraint_names::ConstraintNames, Connector, NativeTypeInstance, ReferentialAction, ReferentialIntegrity,
};
use parser_database::{ast, walkers::*};
use std::borrow::Cow;

pub trait IndexWalkerExt<'ast> {
    fn constraint_name(self, connector: &dyn Connector) -> Cow<'ast, str>;
}

impl<'ast> IndexWalkerExt<'ast> for IndexWalker<'ast, '_> {
    fn constraint_name(self, connector: &dyn Connector) -> Cow<'ast, str> {
        if let Some(mapped_name) = self.mapped_name() {
            return Cow::from(mapped_name);
        }

        let model = self.model();
        let model_db_name = model.database_name();
        let field_db_names: Vec<&str> = model
            .get_field_database_names(&self.fields().map(|f| f.field_id()).collect::<Vec<_>>())
            .collect();

        if self.is_unique() {
            ConstraintNames::unique_index_name(model_db_name, &field_db_names, connector).into()
        } else {
            ConstraintNames::non_unique_index_name(model_db_name, &field_db_names, connector).into()
        }
    }
}

pub trait DefaultValueExt<'ast> {
    fn constraint_name(self, connector: &dyn Connector) -> Cow<'ast, str>;
}

impl<'ast> DefaultValueExt<'ast> for DefaultValueWalker<'ast, '_> {
    fn constraint_name(self, connector: &dyn Connector) -> Cow<'ast, str> {
        self.mapped_name().map(Cow::from).unwrap_or_else(|| {
            let name = ConstraintNames::default_name(
                self.field().model().database_name(),
                self.field().database_name(),
                connector,
            );

            Cow::from(name)
        })
    }
}

pub trait PrimaryKeyWalkerExt<'ast> {
    /// This will be None if and only if the connector does not support named primary keys. It can
    /// be a generated name or one explicitly set in the schema.
    fn constraint_name(self, connector: &dyn Connector) -> Option<Cow<'ast, str>>;
}

impl<'ast> PrimaryKeyWalkerExt<'ast> for PrimaryKeyWalker<'ast, '_> {
    fn constraint_name(self, connector: &dyn Connector) -> Option<Cow<'ast, str>> {
        if !connector.supports_named_primary_keys() {
            return None;
        }

        Some(
            self.mapped_name()
                .map(Cow::Borrowed)
                .unwrap_or_else(|| ConstraintNames::primary_key_name(self.model().database_name(), connector).into()),
        )
    }
}

pub trait CompleteInlineRelationWalkerExt<'ast> {
    /// Gives the onDelete referential action of the relation. If not defined
    /// explicitly, returns the default value.
    fn on_delete(self, connector: &dyn Connector, referential_integrity: ReferentialIntegrity) -> ReferentialAction;
}

impl<'ast> CompleteInlineRelationWalkerExt<'ast> for CompleteInlineRelationWalker<'ast, '_> {
    fn on_delete(self, connector: &dyn Connector, referential_integrity: ReferentialIntegrity) -> ReferentialAction {
        use crate::ReferentialAction::*;

        self.referencing_field().explicit_on_delete().unwrap_or_else(|| {
            let supports_restrict = connector.supports_referential_action(&referential_integrity, Restrict);

            match self.referential_arity() {
                ast::FieldArity::Required if supports_restrict => Restrict,
                ast::FieldArity::Required => NoAction,
                _ => SetNull,
            }
        })
    }
}

pub trait InlineRelationWalkerExt<'db> {
    fn constraint_name(self, connector: &dyn Connector) -> Cow<'db, str>;
}

impl<'db> InlineRelationWalkerExt<'db> for InlineRelationWalker<'_, 'db> {
    fn constraint_name(self, connector: &dyn Connector) -> Cow<'db, str> {
        self.mapped_name().map(Cow::Borrowed).unwrap_or_else(|| {
            let model_database_name = self.referencing_model().database_name();
            let field_names: Vec<&str> = match self.referencing_fields() {
                ReferencingFields::Concrete(fields) => fields.map(|f| f.database_name()).collect(),
                ReferencingFields::Inferred(fields) => {
                    let field_names: Vec<_> = fields.iter().map(|f| f.name.as_str()).collect();
                    return ConstraintNames::foreign_key_constraint_name(model_database_name, &field_names, connector)
                        .into();
                }
                ReferencingFields::NA => Vec::new(),
            };
            ConstraintNames::foreign_key_constraint_name(model_database_name, &field_names, connector).into()
        })
    }
}

pub trait ScalarFieldWalkerExt {
    /// This will return None when:
    ///
    /// - There is no native type attribute on the field.
    /// - The native type attribute is not valid for the connector.
    fn native_type_instance(&self, connector: &dyn Connector) -> Option<NativeTypeInstance>;
}

impl ScalarFieldWalkerExt for ScalarFieldWalker<'_, '_> {
    fn native_type_instance(&self, connector: &dyn Connector) -> Option<NativeTypeInstance> {
        self.raw_native_type()
            .and_then(|(_, name, args, _)| connector.parse_native_type(name, args.to_owned()).ok())
    }
}

pub trait RelationFieldWalkerExt {
    fn default_on_delete_action(self, integrity: ReferentialIntegrity, connector: &dyn Connector) -> ReferentialAction;
}

impl RelationFieldWalkerExt for RelationFieldWalker<'_, '_> {
    fn default_on_delete_action(self, integrity: ReferentialIntegrity, connector: &dyn Connector) -> ReferentialAction {
        match self.referential_arity() {
            ast::FieldArity::Required
                if connector.supports_referential_action(&integrity, ReferentialAction::Restrict) =>
            {
                ReferentialAction::Restrict
            }
            ast::FieldArity::Required => ReferentialAction::NoAction,
            _ => ReferentialAction::SetNull,
        }
    }
}
