use crate::datamodel_connector::{
    constraint_names::ConstraintNames, Connector, NativeTypeInstance, ReferentialAction, ReferentialIntegrity,
};
use parser_database::{
    ast::{self, WithSpan},
    walkers::*,
};
use std::borrow::Cow;

pub trait IndexWalkerExt<'db> {
    fn constraint_name(self, connector: &dyn Connector) -> Cow<'db, str>;
}

impl<'db> IndexWalkerExt<'db> for IndexWalker<'db> {
    fn constraint_name(self, connector: &dyn Connector) -> Cow<'db, str> {
        if let Some(mapped_name) = self.mapped_name() {
            return Cow::from(mapped_name);
        }

        let model = self.model();
        let model_db_name = model.database_name();

        let field_db_names = self
            .scalar_field_attributes()
            .map(|f| f.as_mapped_path_to_indexed_field())
            .collect::<Vec<_>>();

        if self.is_unique() {
            ConstraintNames::unique_index_name(model_db_name, &field_db_names, connector).into()
        } else {
            ConstraintNames::non_unique_index_name(model_db_name, &field_db_names, connector).into()
        }
    }
}

pub trait DefaultValueExt<'db> {
    fn constraint_name(self, connector: &dyn Connector) -> Cow<'db, str>;
}

impl<'db> DefaultValueExt<'db> for DefaultValueWalker<'db> {
    fn constraint_name(self, connector: &dyn Connector) -> Cow<'db, str> {
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

pub trait PrimaryKeyWalkerExt<'db> {
    /// This will be None if and only if the connector does not support named primary keys. It can
    /// be a generated name or one explicitly set in the schema.
    fn constraint_name(self, connector: &dyn Connector) -> Option<Cow<'db, str>>;
}

impl<'db> PrimaryKeyWalkerExt<'db> for PrimaryKeyWalker<'db> {
    fn constraint_name(self, connector: &dyn Connector) -> Option<Cow<'db, str>> {
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

pub trait CompleteInlineRelationWalkerExt<'db> {
    /// Gives the onDelete referential action of the relation. If not defined
    /// explicitly, returns the default value.
    fn on_delete(self, connector: &dyn Connector, referential_integrity: ReferentialIntegrity) -> ReferentialAction;
}

impl<'db> CompleteInlineRelationWalkerExt<'db> for CompleteInlineRelationWalker<'db> {
    fn on_delete(self, connector: &dyn Connector, referential_integrity: ReferentialIntegrity) -> ReferentialAction {
        use ReferentialAction::*;

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

impl<'db> InlineRelationWalkerExt<'db> for InlineRelationWalker<'db> {
    fn constraint_name(self, connector: &dyn Connector) -> Cow<'db, str> {
        self.mapped_name().map(Cow::Borrowed).unwrap_or_else(|| {
            let model_database_name = self.referencing_model().database_name();
            let field_names: Vec<&str> = self
                .referencing_fields()
                .map(|fields| fields.map(|f| f.database_name()).collect())
                .unwrap_or_default();
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

impl ScalarFieldWalkerExt for ScalarFieldWalker<'_> {
    fn native_type_instance(&self, connector: &dyn Connector) -> Option<NativeTypeInstance> {
        self.raw_native_type().and_then(|(_, name, args, _)| {
            connector
                .parse_native_type(name, args.to_owned(), self.ast_field().span())
                .ok()
        })
    }
}

impl ScalarFieldWalkerExt for CompositeTypeFieldWalker<'_> {
    fn native_type_instance(&self, connector: &dyn Connector) -> Option<NativeTypeInstance> {
        self.raw_native_type().and_then(|(_, name, args, _)| {
            connector
                .parse_native_type(name, args.to_owned(), self.ast_field().span())
                .ok()
        })
    }
}

impl ScalarFieldWalkerExt for IndexFieldWalker<'_> {
    fn native_type_instance(&self, connector: &dyn Connector) -> Option<NativeTypeInstance> {
        self.raw_native_type().and_then(|(_, name, args, _)| {
            connector
                .parse_native_type(name, args.to_owned(), self.ast_field().span())
                .ok()
        })
    }
}

pub trait RelationFieldWalkerExt {
    fn default_on_delete_action(self, integrity: ReferentialIntegrity, connector: &dyn Connector) -> ReferentialAction;
}

impl RelationFieldWalkerExt for RelationFieldWalker<'_> {
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
