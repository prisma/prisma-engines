use crate::{constraint_names::ConstraintNames, Connector, ReferentialAction, ReferentialIntegrity};
use parser_database::{ast, walkers::*, ScalarType};
use std::borrow::Cow;

pub trait IndexWalkerExt<'ast> {
    fn final_database_name(self, connector: &dyn Connector) -> Cow<'ast, str>;
}

impl<'ast> IndexWalkerExt<'ast> for IndexWalker<'ast, '_> {
    fn final_database_name(self, connector: &dyn Connector) -> Cow<'ast, str> {
        if let Some(mapped_name) = self.mapped_name() {
            return Cow::from(mapped_name);
        }

        let model = self.model();
        let model_db_name = model.final_database_name();
        let field_db_names: Vec<&str> = model
            .get_field_db_names(&self.fields().map(|f| f.field_id()).collect::<Vec<_>>())
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
    fn dml_default_kind(self) -> dml::default_value::DefaultKind;
}

impl<'ast> DefaultValueExt<'ast> for DefaultValueWalker<'ast, '_> {
    fn dml_default_kind(self) -> dml::default_value::DefaultKind {
        use dml::{
            default_value::{DefaultKind, ValueGenerator},
            prisma_value::PrismaValue,
        };

        // This has all been validated in parser-database, so unwrapping is always safe.
        match self.value() {
            ast::Expression::Function(funcname, args, _) if funcname == "dbgenerated" => {
                DefaultKind::Expression(ValueGenerator::new_dbgenerated(
                    args.get(0)
                        .and_then(|arg| arg.as_string_value())
                        .map(|(val, _)| val.to_owned())
                        .unwrap_or_else(String::new),
                ))
            }
            ast::Expression::Function(funcname, _args, _) if funcname == "autoincrement" => {
                DefaultKind::Expression(ValueGenerator::new_autoincrement())
            }
            ast::Expression::Function(funcname, _args, _) if funcname == "uuid" => {
                DefaultKind::Expression(ValueGenerator::new_uuid())
            }
            ast::Expression::Function(funcname, _args, _) if funcname == "cuid" => {
                DefaultKind::Expression(ValueGenerator::new_cuid())
            }
            ast::Expression::Function(funcname, _args, _) if funcname == "now" => {
                DefaultKind::Expression(ValueGenerator::new_now())
            }
            ast::Expression::NumericValue(num, _) => match self.field().scalar_type() {
                Some(ScalarType::Int) => DefaultKind::Single(PrismaValue::Int(num.parse().unwrap())),
                Some(ScalarType::BigInt) => DefaultKind::Single(PrismaValue::BigInt(num.parse().unwrap())),
                Some(ScalarType::Float) => DefaultKind::Single(PrismaValue::Float(num.parse().unwrap())),
                Some(ScalarType::Decimal) => DefaultKind::Single(PrismaValue::Float(num.parse().unwrap())),
                other => unreachable!("{:?}", other),
            },
            ast::Expression::ConstantValue(v, _) => match self.field().scalar_type() {
                Some(ScalarType::Boolean) => DefaultKind::Single(PrismaValue::Boolean(v.parse().unwrap())),
                None => DefaultKind::Single(PrismaValue::Enum(v.to_owned())),
                other => unreachable!("{:?}", other),
            },
            ast::Expression::StringValue(v, _) => match self.field().scalar_type() {
                Some(ScalarType::DateTime) => DefaultKind::Single(PrismaValue::DateTime(v.parse().unwrap())),
                Some(ScalarType::String) => DefaultKind::Single(PrismaValue::String(v.parse().unwrap())),
                Some(ScalarType::Json) => DefaultKind::Single(PrismaValue::Json(v.parse().unwrap())),
                Some(ScalarType::Decimal) => DefaultKind::Single(PrismaValue::Float(v.parse().unwrap())),
                Some(ScalarType::Bytes) => {
                    DefaultKind::Single(PrismaValue::Bytes(dml::prisma_value::decode_bytes(v).unwrap()))
                }
                other => unreachable!("{:?}", other),
            },
            other => unreachable!("{:?}", other),
        }
    }

    fn constraint_name(self, connector: &dyn Connector) -> Cow<'ast, str> {
        self.mapped_name().map(Cow::from).unwrap_or_else(|| {
            let name = ConstraintNames::default_name(
                self.field().model().final_database_name(),
                self.field().database_name(),
                connector,
            );

            Cow::from(name)
        })
    }
}

pub trait PrimaryKeyWalkerExt<'ast> {
    fn final_database_name(self, connector: &dyn Connector) -> Option<Cow<'ast, str>>;
}

impl<'ast> PrimaryKeyWalkerExt<'ast> for PrimaryKeyWalker<'ast, '_> {
    fn final_database_name(self, connector: &dyn Connector) -> Option<Cow<'ast, str>> {
        if !connector.supports_named_primary_keys() {
            return None;
        }

        Some(
            self.mapped_name().map(Cow::Borrowed).unwrap_or_else(|| {
                ConstraintNames::primary_key_name(self.model().final_database_name(), connector).into()
            }),
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

pub trait InlineRelationWalkerExt<'ast> {
    fn constraint_name(self, connector: &dyn Connector) -> Cow<'ast, str>;
}

impl<'ast> InlineRelationWalkerExt<'ast> for InlineRelationWalker<'ast, '_> {
    fn constraint_name(self, connector: &dyn Connector) -> Cow<'ast, str> {
        self.foreign_key_name().map(Cow::Borrowed).unwrap_or_else(|| {
            let model_database_name = self.referencing_model().final_database_name();
            match self.referencing_fields() {
                ReferencingFields::Concrete(fields) => {
                    let field_names: Vec<&str> = fields.map(|f| f.database_name()).collect();
                    ConstraintNames::foreign_key_constraint_name(model_database_name, &field_names, connector).into()
                }
                ReferencingFields::Inferred(fields) => {
                    let field_names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
                    ConstraintNames::foreign_key_constraint_name(model_database_name, &field_names, connector).into()
                }
                ReferencingFields::NA => unreachable!(),
            }
        })
    }
}

pub trait ScalarFieldWalkerExt {
    /// This will return None when:
    ///
    /// - There is no native type attribute on the field.
    /// - The native type attribute is not valid for the connector.
    fn native_type_instance(&self, connector: &dyn Connector) -> Option<dml::native_type_instance::NativeTypeInstance>;
}

impl ScalarFieldWalkerExt for ScalarFieldWalker<'_, '_> {
    fn native_type_instance(&self, connector: &dyn Connector) -> Option<dml::native_type_instance::NativeTypeInstance> {
        self.raw_native_type()
            .and_then(|(_, name, args, _)| connector.parse_native_type(name, args.to_owned()).ok())
    }
}
