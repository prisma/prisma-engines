use std::fmt::Debug;

use either::Either::{Left, Right};
use psl::datamodel_connector::Connector;
use psl::diagnostics::DatamodelWarning;
use psl::parser_database::{walkers, IndexAlgorithm, OperatorClass, ReferentialAction, ScalarType, SortOrder};
use psl::schema_ast::ast::WithDocumentation;
use psl::schema_ast::ast::{self, FieldArity};
use psl::{Diagnostics, StringFromEnvVar};

pub(crate) trait DatamodelAssert<'a> {
    fn assert_has_model(&'a self, name: &str) -> walkers::ModelWalker<'a>;
    fn assert_has_type(&'a self, name: &str) -> walkers::CompositeTypeWalker<'a>;
}

pub(crate) trait DatasourceAsserts {
    fn assert_name(&self, name: &str) -> &Self;
    fn assert_url(&self, url: StringFromEnvVar) -> &Self;
}

pub(crate) trait WarningAsserts {
    fn assert_is(&self, warning: DatamodelWarning) -> &Self;
}

pub(crate) trait ModelAssert<'a> {
    fn assert_field_count(&self, count: usize) -> &Self;
    fn assert_has_scalar_field(&self, t: &str) -> walkers::ScalarFieldWalker<'a>;
    fn assert_has_relation_field(&self, name: &str) -> walkers::RelationFieldWalker<'a>;
    fn assert_ignored(&self, ignored: bool) -> &Self;
    fn assert_with_documentation(&self, t: &str) -> &Self;
    fn assert_index_on_fields(&self, fields: &[&str]) -> walkers::IndexWalker<'a>;
    fn assert_unique_on_fields(&self, fields: &[&str]) -> walkers::IndexWalker<'a>;
    fn assert_unique_on_fields_and_name(&self, fields: &[&str], name: &str) -> walkers::IndexWalker<'a>;
    fn assert_fulltext_on_fields(&self, fields: &[&str]) -> walkers::IndexWalker<'a>;
    fn assert_id_on_fields(&self, fields: &[&str]) -> walkers::PrimaryKeyWalker<'a>;
    fn assert_mapped_name(&self, name: &str) -> &Self;
}

pub(crate) trait TypeAssert<'a> {
    fn assert_has_scalar_field(&self, t: &str) -> walkers::CompositeTypeFieldWalker<'a>;
}

pub(crate) trait ScalarFieldAssert {
    fn assert_scalar_type(&self, t: ScalarType) -> &Self;
    fn assert_is_single_field_id(&self) -> walkers::PrimaryKeyWalker<'_>;
    fn assert_is_single_field_unique(&self) -> walkers::IndexWalker<'_>;
    fn assert_not_single_field_unique(&self) -> &Self;
    fn assert_ignored(&self, ignored: bool) -> &Self;
    fn assert_with_documentation(&self, t: &str) -> &Self;
    fn assert_required(&self) -> &Self;
    fn assert_optional(&self) -> &Self;
    fn assert_list(&self) -> &Self;
    fn assert_default_value(&self) -> walkers::DefaultValueWalker<'_>;
    fn assert_mapped_name(&self, name: &str) -> &Self;
    fn assert_is_updated_at(&self) -> &Self;

    fn assert_native_type<T>(&self, connector: &dyn Connector, typ: &T) -> &Self
    where
        T: Debug + PartialEq + 'static;
}

pub(crate) trait CompositeFieldAssert {
    fn assert_scalar_type(&self, t: ScalarType) -> &Self;
    fn assert_default_value(&self) -> &ast::Expression;
    fn assert_mapped_name(&self, name: &str) -> &Self;
}

pub(crate) trait RelationFieldAssert {
    fn assert_ignored(&self, ignored: bool) -> &Self;
    fn assert_relation_to(&self, model_id: ast::ModelId) -> &Self;
    fn assert_relation_delete_strategy(&self, action: ReferentialAction) -> &Self;
    fn assert_relation_update_strategy(&self, action: ReferentialAction) -> &Self;
}

pub(crate) trait DefaultValueAssert {
    fn assert_autoincrement(&self) -> &Self;
    fn assert_auto(&self) -> &Self;
    fn assert_string(&self, val: &str) -> &Self;
    fn assert_int(&self, val: usize) -> &Self;
    fn assert_decimal(&self, val: &str) -> &Self;
    fn assert_bool(&self, val: bool) -> &Self;
    fn assert_constant(&self, val: &str) -> &Self;
    fn assert_bytes(&self, val: &[u8]) -> &Self;
    fn assert_now(&self) -> &Self;
    fn assert_cuid(&self) -> &Self;
    fn assert_uuid(&self) -> &Self;
    fn assert_dbgenerated(&self, val: &str) -> &Self;
    fn assert_mapped_name(&self, val: &str) -> &Self;
}

pub(crate) trait IndexAssert {
    fn assert_field(&self, name: &str) -> walkers::ScalarFieldAttributeWalker<'_>;
    fn assert_name(&self, name: &str) -> &Self;
    fn assert_mapped_name(&self, name: &str) -> &Self;
    fn assert_clustered(&self, clustered: bool) -> &Self;
    fn assert_type(&self, r#type: IndexAlgorithm) -> &Self;
}

pub(crate) trait IndexFieldAssert {
    fn assert_descending(&self) -> &Self;
    fn assert_length(&self, length: u32) -> &Self;
    fn assert_ops(&self, ops: OperatorClass) -> &Self;
    fn assert_raw_ops(&self, ops: &str) -> &Self;
}

impl DatasourceAsserts for psl::Datasource {
    #[track_caller]
    fn assert_name(&self, name: &str) -> &Self {
        assert_eq!(&self.name, name);
        self
    }

    #[track_caller]
    fn assert_url(&self, url: StringFromEnvVar) -> &Self {
        assert_eq!(self.url, url);
        self
    }
}

impl WarningAsserts for Vec<DatamodelWarning> {
    #[track_caller]
    fn assert_is(&self, warning: DatamodelWarning) -> &Self {
        assert_eq!(
            self.len(),
            1,
            "Expected exactly one validation warning. Warnings are: {:?}",
            &self
        );
        assert_eq!(self[0], warning);
        self
    }
}

impl<'a> DatamodelAssert<'a> for psl::ValidatedSchema {
    #[track_caller]
    fn assert_has_model(&'a self, name: &str) -> walkers::ModelWalker<'a> {
        self.db
            .walk_models()
            .find(|m| m.name() == name)
            .expect("Model {name} not found")
    }

    #[track_caller]
    fn assert_has_type(&'a self, name: &str) -> walkers::CompositeTypeWalker<'a> {
        self.db
            .walk_composite_types()
            .find(|m| m.name() == name)
            .expect("Type {name} not found")
    }
}

impl<'a> RelationFieldAssert for walkers::RelationFieldWalker<'a> {
    #[track_caller]
    fn assert_relation_to(&self, model_id: ast::ModelId) -> &Self {
        assert!(self.references_model(model_id));
        self
    }

    #[track_caller]
    fn assert_ignored(&self, ignored: bool) -> &Self {
        assert_eq!(self.is_ignored(), ignored);
        self
    }

    #[track_caller]
    fn assert_relation_delete_strategy(&self, action: ReferentialAction) -> &Self {
        assert_eq!(self.explicit_on_delete(), Some(action));
        self
    }

    #[track_caller]
    fn assert_relation_update_strategy(&self, action: ReferentialAction) -> &Self {
        assert_eq!(self.explicit_on_update(), Some(action));
        self
    }
}

impl<'a> ModelAssert<'a> for walkers::ModelWalker<'a> {
    #[track_caller]
    fn assert_field_count(&self, count: usize) -> &Self {
        assert_eq!(self.scalar_fields().count() + self.relation_fields().count(), count);
        self
    }

    #[track_caller]
    fn assert_ignored(&self, ignored: bool) -> &Self {
        assert_eq!(self.is_ignored(), ignored);
        self
    }

    #[track_caller]
    fn assert_has_relation_field(&self, t: &str) -> walkers::RelationFieldWalker<'a> {
        self.relation_fields()
            .find(|sf| sf.name() == t)
            .expect("Could not find scalar field with name {t}")
    }

    #[track_caller]
    fn assert_has_scalar_field(&self, t: &str) -> walkers::ScalarFieldWalker<'a> {
        self.scalar_fields()
            .find(|sf| sf.name() == t)
            .expect("Could not find scalar field with name {t}")
    }

    #[track_caller]
    fn assert_with_documentation(&self, t: &str) -> &Self {
        assert_eq!(Some(t), self.ast_model().documentation());
        self
    }

    #[track_caller]
    fn assert_index_on_fields(&self, fields: &[&str]) -> walkers::IndexWalker<'a> {
        self.indexes()
            .filter(|i| i.is_normal())
            .find(|i| i.fields().len() == fields.len() && i.fields().zip(fields).all(|(a, b)| a.name() == *b))
            .expect("Could not find index with the given fields.")
    }

    #[track_caller]
    fn assert_unique_on_fields(&self, fields: &[&str]) -> walkers::IndexWalker<'a> {
        self.indexes()
            .filter(|i| i.is_unique())
            .find(|i| i.fields().len() == fields.len() && i.fields().zip(fields).all(|(a, b)| a.name() == *b))
            .expect("Could not find index with the given fields.")
    }

    #[track_caller]
    fn assert_unique_on_fields_and_name(&self, fields: &[&str], name: &str) -> walkers::IndexWalker<'a> {
        self.indexes()
            .filter(|i| i.is_unique())
            .find(|i| {
                i.name() == Some(name)
                    && i.fields().len() == fields.len()
                    && i.fields().zip(fields).all(|(a, b)| a.name() == *b)
            })
            .expect("Could not find index with the given fields.")
    }

    #[track_caller]
    fn assert_fulltext_on_fields(&self, fields: &[&str]) -> walkers::IndexWalker<'a> {
        self.indexes()
            .filter(|i| i.is_fulltext())
            .find(|i| i.fields().len() == fields.len() && i.fields().zip(fields).all(|(a, b)| a.name() == *b))
            .expect("Could not find index with the given fields.")
    }

    #[track_caller]
    fn assert_id_on_fields(&self, fields: &[&str]) -> walkers::PrimaryKeyWalker<'a> {
        self.primary_key()
            .filter(|pk| {
                let pk_fields = pk.fields();
                pk_fields.len() == fields.len() && pk_fields.zip(fields).all(|(a, b)| a.name() == *b)
            })
            .expect("Model does not have a primary key with the given fields")
    }

    #[track_caller]
    fn assert_mapped_name(&self, name: &str) -> &Self {
        assert_eq!(Some(name), self.mapped_name());
        self
    }
}

impl<'a> ScalarFieldAssert for walkers::ScalarFieldWalker<'a> {
    #[track_caller]
    fn assert_ignored(&self, ignored: bool) -> &Self {
        assert_eq!(self.is_ignored(), ignored);
        self
    }

    #[track_caller]
    fn assert_mapped_name(&self, name: &str) -> &Self {
        assert_eq!(Some(name), self.mapped_name());
        self
    }

    #[track_caller]
    fn assert_scalar_type(&self, t: ScalarType) -> &Self {
        assert_eq!(self.scalar_type(), Some(t));
        self
    }

    #[track_caller]
    fn assert_required(&self) -> &Self {
        assert_eq!(FieldArity::Required, self.ast_field().arity);
        self
    }

    #[track_caller]
    fn assert_optional(&self) -> &Self {
        assert_eq!(FieldArity::Optional, self.ast_field().arity);
        self
    }

    #[track_caller]
    fn assert_list(&self) -> &Self {
        assert_eq!(FieldArity::List, self.ast_field().arity);
        self
    }

    #[track_caller]
    fn assert_is_single_field_id(&self) -> walkers::PrimaryKeyWalker<'_> {
        self.model()
            .primary_key()
            .filter(|id| id.is_defined_on_field())
            .filter(|id| id.contains_exactly_fields(std::iter::once(*self)))
            .expect("Field is not a single-field id.")
    }

    #[track_caller]
    fn assert_is_single_field_unique(&self) -> walkers::IndexWalker<'_> {
        self.model()
            .indexes()
            .filter(|i| i.is_defined_on_field())
            .filter(|i| i.is_unique())
            .find(|i| i.contains_field(*self))
            .expect("Field is not a single-field unique.")
    }

    #[track_caller]
    fn assert_not_single_field_unique(&self) -> &Self {
        match self
            .model()
            .indexes()
            .filter(|i| i.is_defined_on_field())
            .filter(|i| i.is_unique())
            .find(|i| i.contains_field(*self))
        {
            Some(_) => panic!("Expected field to not be part of a unique index."),
            None => self,
        }
    }

    #[track_caller]
    fn assert_with_documentation(&self, t: &str) -> &Self {
        assert_eq!(Some(t), self.ast_field().documentation());
        self
    }

    #[track_caller]
    fn assert_native_type<T>(&self, connector: &dyn Connector, typ: &T) -> &Self
    where
        T: Debug + PartialEq + 'static,
    {
        let (_, r#type, params, span) = match self.raw_native_type() {
            Some(tuple) => tuple,
            None => panic!("Field does not have native type set."),
        };

        let mut diagnostics = Diagnostics::new();

        let nt = match connector.parse_native_type(r#type, params, span, &mut diagnostics) {
            Some(nt) => nt,
            None => panic!("Invalid native type {}", r#type),
        };

        diagnostics.to_result().unwrap();
        assert_eq!(typ, nt.downcast_ref());

        self
    }

    #[track_caller]
    fn assert_default_value(&self) -> walkers::DefaultValueWalker<'_> {
        self.default_value().expect("Field does not have a default value")
    }

    #[track_caller]
    fn assert_is_updated_at(&self) -> &Self {
        assert!(self.is_updated_at());
        self
    }
}

impl<'a> DefaultValueAssert for walkers::DefaultValueWalker<'a> {
    #[track_caller]
    fn assert_autoincrement(&self) -> &Self {
        self.value().assert_autoincrement();
        self
    }

    #[track_caller]
    fn assert_auto(&self) -> &Self {
        self.value().assert_auto();
        self
    }

    #[track_caller]
    fn assert_string(&self, expected: &str) -> &Self {
        self.value().assert_string(expected);
        self
    }

    #[track_caller]
    fn assert_int(&self, expected: usize) -> &Self {
        self.value().assert_int(expected);
        self
    }

    #[track_caller]
    fn assert_decimal(&self, expected: &str) -> &Self {
        self.value().assert_decimal(expected);
        self
    }

    #[track_caller]
    fn assert_bool(&self, expected: bool) -> &Self {
        self.value().assert_bool(expected);
        self
    }

    #[track_caller]
    fn assert_constant(&self, expected: &str) -> &Self {
        self.value().assert_constant(expected);
        self
    }

    #[track_caller]
    fn assert_bytes(&self, expected: &[u8]) -> &Self {
        self.value().assert_bytes(expected);
        self
    }

    #[track_caller]
    fn assert_now(&self) -> &Self {
        self.value().assert_now();
        self
    }

    #[track_caller]
    fn assert_cuid(&self) -> &Self {
        self.value().assert_cuid();
        self
    }

    #[track_caller]
    fn assert_uuid(&self) -> &Self {
        self.value().assert_uuid();
        self
    }

    #[track_caller]
    fn assert_dbgenerated(&self, val: &str) -> &Self {
        self.value().assert_dbgenerated(val);
        self
    }

    #[track_caller]
    fn assert_mapped_name(&self, name: &str) -> &Self {
        assert_eq!(Some(name), self.mapped_name());
        self
    }
}

impl<'a> IndexAssert for walkers::IndexWalker<'a> {
    #[track_caller]
    fn assert_field(&self, name: &str) -> walkers::ScalarFieldAttributeWalker<'_> {
        self.scalar_field_attributes()
            .find(|f| f.as_index_field().name() == name)
            .expect("Could not find an index field.")
    }

    #[track_caller]
    fn assert_name(&self, name: &str) -> &Self {
        if self.name().is_some() {
            assert_eq!(Some(name), self.name());
        } else {
            assert_eq!(Some(name), self.mapped_name());
        }

        self
    }

    #[track_caller]
    fn assert_mapped_name(&self, name: &str) -> &Self {
        assert_eq!(Some(name), self.mapped_name());
        self
    }

    #[track_caller]
    fn assert_clustered(&self, clustered: bool) -> &Self {
        assert_eq!(Some(clustered), self.clustered());
        self
    }

    #[track_caller]
    fn assert_type(&self, r#type: IndexAlgorithm) -> &Self {
        assert_eq!(Some(r#type), self.algorithm());
        self
    }
}

impl<'a> IndexFieldAssert for walkers::ScalarFieldAttributeWalker<'a> {
    #[track_caller]
    fn assert_descending(&self) -> &Self {
        assert_eq!(Some(SortOrder::Desc), self.sort_order());
        self
    }

    #[track_caller]
    fn assert_length(&self, length: u32) -> &Self {
        assert_eq!(Some(length), self.length());
        self
    }

    #[track_caller]
    fn assert_ops(&self, ops: OperatorClass) -> &Self {
        assert_eq!(Some(Left(ops)), self.operator_class().map(|ops| ops.get()));
        self
    }

    #[track_caller]
    fn assert_raw_ops(&self, ops: &str) -> &Self {
        assert_eq!(Some(Right(ops)), self.operator_class().map(|ops| ops.get()));
        self
    }
}

impl<'a> TypeAssert<'a> for walkers::CompositeTypeWalker<'a> {
    #[track_caller]
    fn assert_has_scalar_field(&self, t: &str) -> walkers::CompositeTypeFieldWalker<'a> {
        self.fields()
            .find(|f| f.name() == t)
            .expect("Could not find field {t}.")
    }
}

impl<'a> CompositeFieldAssert for walkers::CompositeTypeFieldWalker<'a> {
    #[track_caller]
    fn assert_scalar_type(&self, t: ScalarType) -> &Self {
        assert_eq!(Some(t), self.scalar_type());
        self
    }

    #[track_caller]
    fn assert_mapped_name(&self, t: &str) -> &Self {
        assert_eq!(Some(t), self.mapped_name());
        self
    }

    #[track_caller]
    fn assert_default_value(&self) -> &ast::Expression {
        self.default_value().expect("Field does not have a default value")
    }
}

impl DefaultValueAssert for ast::Expression {
    #[track_caller]
    fn assert_autoincrement(&self) -> &Self {
        assert!(matches!(self, ast::Expression::Function(name, _, _) if name == "autoincrement"));

        self
    }

    #[track_caller]
    fn assert_auto(&self) -> &Self {
        assert!(matches!(self, ast::Expression::Function(name, _, _) if name == "auto"));

        self
    }

    #[track_caller]
    fn assert_string(&self, expected: &str) -> &Self {
        match self {
            ast::Expression::StringValue(actual, _) => assert_eq!(actual, expected),
            _ => panic!("Not a string value"),
        }

        self
    }

    #[track_caller]
    fn assert_int(&self, expected: usize) -> &Self {
        match self {
            ast::Expression::NumericValue(actual, _) => assert_eq!(actual, &format!("{expected}")),
            _ => panic!("Not a number value"),
        }

        self
    }

    #[track_caller]
    fn assert_decimal(&self, expected: &str) -> &Self {
        match self {
            ast::Expression::StringValue(actual, _) => assert_eq!(actual, expected),
            _ => panic!("Not a decimal value"),
        }

        self
    }

    #[track_caller]
    fn assert_bool(&self, expected: bool) -> &Self {
        assert!(matches!(self, ast::Expression::ConstantValue(actual, _) if actual == &format!("{expected}")));

        self
    }

    #[track_caller]
    fn assert_constant(&self, expected: &str) -> &Self {
        assert!(matches!(self, ast::Expression::ConstantValue(actual, _) if actual == expected));

        self
    }

    #[track_caller]
    fn assert_bytes(&self, expected: &[u8]) -> &Self {
        match self {
            ast::Expression::StringValue(actual, _) => assert_eq!(base64::decode(actual).unwrap(), expected),
            _ => panic!("Not a bytes value"),
        }

        self
    }

    #[track_caller]
    fn assert_now(&self) -> &Self {
        assert!(matches!(self, ast::Expression::Function(name, args, _) if name == "now" && args.arguments.is_empty()));

        self
    }

    #[track_caller]
    fn assert_cuid(&self) -> &Self {
        assert!(
            matches!(self, ast::Expression::Function(name, args, _) if name == "cuid" && args.arguments.is_empty())
        );

        self
    }

    #[track_caller]
    fn assert_uuid(&self) -> &Self {
        assert!(
            matches!(self, ast::Expression::Function(name, args, _) if name == "uuid" && args.arguments.is_empty())
        );

        self
    }

    #[track_caller]
    fn assert_dbgenerated(&self, val: &str) -> &Self {
        match self {
            ast::Expression::Function(name, args, _) if name == "dbgenerated" => args
                .arguments
                .first()
                .expect("Expected a parameter for dbgenerated.")
                .value
                .assert_string(val),
            _ => panic!("Expected a dbgenerated function."),
        }
    }

    #[track_caller]
    fn assert_mapped_name(&self, _name: &str) -> &Self {
        unreachable!()
    }
}

impl<'a> IndexAssert for walkers::PrimaryKeyWalker<'a> {
    #[track_caller]
    fn assert_field(&self, name: &str) -> walkers::ScalarFieldAttributeWalker<'_> {
        self.scalar_field_attributes()
            .find(|f| f.as_index_field().name() == name)
            .expect("Could not find a field with the given name")
    }

    #[track_caller]
    fn assert_name(&self, name: &str) -> &Self {
        assert_eq!(Some(name), self.name());
        self
    }

    #[track_caller]
    fn assert_mapped_name(&self, name: &str) -> &Self {
        assert_eq!(Some(name), self.mapped_name());
        self
    }

    #[track_caller]
    fn assert_clustered(&self, clustered: bool) -> &Self {
        assert_eq!(Some(clustered), self.clustered());
        self
    }

    #[track_caller]
    fn assert_type(&self, _type: IndexAlgorithm) -> &Self {
        unreachable!("Primary key cannot define the index type.");
    }
}
