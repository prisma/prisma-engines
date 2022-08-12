use crate::parser_database::{
    self as db,
    ast::{self, WithDocumentation, WithName, WithSpan},
    walkers::*,
    IndexAlgorithm,
};
use datamodel_connector::{walker_ext_traits::*, Connector, ReferentialIntegrity, ScalarType};
use dml::*;
use either::Either;
use std::collections::HashMap;

/// Helper for lifting a datamodel.
///
/// When lifting, the AST is converted to the Datamodel data structure, and
/// additional semantics are attached.
///
/// ## Guarantees
///
/// For a parsed, validated and standardised datamodel, the following guarantees hold:
///
/// - Each referenced model or enum does exist.
/// - Each related field has a backwards related field on the related type with equal relation
/// name. If the user did not specify any, a backwards field will be generated.
/// - All relations are named.
/// - All relations have a valid list of `to_fields` on the referencing side. An empty list
/// indicates the back relation field. If the user does not give any `references` argument, the
/// `to_fields` will point to the related types id fields.
pub(crate) struct LiftAstToDml<'a> {
    db: &'a db::ParserDatabase,
    connector: &'static dyn Connector,
    referential_integrity: ReferentialIntegrity,
}

impl<'a> LiftAstToDml<'a> {
    pub(crate) fn new(
        db: &'a db::ParserDatabase,
        connector: &'static dyn Connector,
        referential_integrity: ReferentialIntegrity,
    ) -> LiftAstToDml<'a> {
        LiftAstToDml {
            db,
            connector,
            referential_integrity,
        }
    }

    pub(crate) fn lift(&self) -> dml::Datamodel {
        let mut schema = dml::Datamodel::new();

        // We iterate over scalar fields, then relations, but we want the
        // order of fields in the dml::Model to match the order of the fields in
        // the AST, so we need this bit of extra bookkeeping.
        //
        // (model_idx, field_name) -> sort_key
        let mut field_ids_for_sorting: HashMap<(&str, &str), ast::FieldId> = HashMap::new();

        for model in self.db.walk_models() {
            schema.add_model(self.lift_model(model, &mut field_ids_for_sorting));
        }

        for composite_type in self.db.walk_composite_types() {
            schema.composite_types.push(self.lift_composite_type(composite_type))
        }

        for r#enum in self.db.walk_enums() {
            schema.add_enum(self.lift_enum(r#enum))
        }

        self.lift_relations(&mut schema, &mut field_ids_for_sorting);

        for model in schema.models_mut() {
            let model_name = model.name.as_str();
            model
                .fields
                .sort_by_key(|field| field_ids_for_sorting.get(&(model_name, field.name())).cloned());
        }

        schema
    }

    fn lift_relations(
        &self,
        schema: &mut dml::Datamodel,
        field_ids_for_sorting: &mut HashMap<(&'a str, &'a str), ast::FieldId>,
    ) {
        let active_connector = self.connector;
        let referential_integrity = self.referential_integrity;
        let common_dml_fields = |field: &mut dml::RelationField, relation_field: RelationFieldWalker<'_>| {
            let ast_field = relation_field.ast_field();
            field.relation_info.on_delete = relation_field
                .explicit_on_delete()
                .map(parser_database_referential_action_to_dml_referential_action);
            field.relation_info.on_update = relation_field
                .explicit_on_update()
                .map(parser_database_referential_action_to_dml_referential_action);
            field.relation_info.name = relation_field.relation_name().to_string();
            field.documentation = ast_field.documentation().map(String::from);
            field.is_ignored = relation_field.is_ignored();
            field.supports_restrict_action(
                active_connector
                    .supports_referential_action(&referential_integrity, parser_database::ReferentialAction::Restrict),
            );
            field.emulates_referential_actions(referential_integrity.is_prisma());
        };

        for relation in self.db.walk_relations() {
            match relation.refine() {
                RefinedRelationWalker::Inline(relation) => {
                    // Forward field
                    {
                        // If we have an array field we detect as a
                        // back-relation, but it has fields defined, we expect
                        // it to be the other side of a embedded 2-way m:n
                        // relation, and we don't want to mess around with the
                        // data model here at all.
                        //
                        // Please kill this with fire when we introduce code
                        // actions for relations.
                        if relation
                            .back_relation_field()
                            .filter(|rf| rf.ast_field().arity.is_list())
                            .and_then(|rf| rf.fields())
                            .is_some()
                        {
                            continue;
                        }

                        let relation_info = dml::RelationInfo::new(relation.referenced_model().name());

                        let forward_field_walker = relation.forward_relation_field().unwrap();
                        // Construct a relation field in the DML for an existing relation field in the source.
                        let arity = self.lift_arity(&forward_field_walker.ast_field().arity);
                        let referential_arity = self.lift_arity(&forward_field_walker.referential_arity());
                        let mut relation_field = dml::RelationField::new(
                            forward_field_walker.name(),
                            arity,
                            referential_arity,
                            relation_info,
                        );

                        relation_field.relation_info.fk_name =
                            Some(relation.constraint_name(active_connector).into_owned());
                        common_dml_fields(&mut relation_field, forward_field_walker);
                        field_ids_for_sorting.insert(
                            (forward_field_walker.model().name(), forward_field_walker.name()),
                            forward_field_walker.field_id(),
                        );

                        relation_field.relation_info.name = relation.relation_name().to_string();

                        relation_field.relation_info.references = relation
                            .referenced_fields()
                            .map(|field| field.name().to_owned())
                            .collect();

                        relation_field.relation_info.fields = relation
                            .referencing_fields()
                            .unwrap()
                            .map(|f| f.name().to_owned())
                            .collect();

                        let model = schema.find_model_mut(relation.referencing_model().name());
                        model.add_field(dml::Field::RelationField(relation_field));
                    };

                    // Back field
                    {
                        let relation_info = dml::RelationInfo::new(relation.referencing_model().name());
                        let model = schema.find_model_mut(relation.referenced_model().name());

                        let mut field = if let Some(relation_field) = relation.back_relation_field() {
                            let ast_field = relation_field.ast_field();
                            let arity = self.lift_arity(&ast_field.arity);
                            let referential_arity = self.lift_arity(&relation_field.referential_arity());
                            let mut field =
                                dml::RelationField::new(relation_field.name(), arity, referential_arity, relation_info);

                            common_dml_fields(&mut field, relation_field);

                            field_ids_for_sorting.insert(
                                (relation_field.model().name(), relation_field.name()),
                                relation_field.field_id(),
                            );

                            field
                        } else {
                            // This is part of reformatting.
                            let arity = dml::FieldArity::List;
                            let referential_arity = dml::FieldArity::List;
                            let mut field = dml::RelationField::new(
                                relation.referencing_model().name(),
                                arity,
                                referential_arity,
                                relation_info,
                            );
                            field.is_ignored = relation.referencing_model().is_ignored();
                            field
                        };

                        field.relation_info.name = relation.relation_name().to_string();
                        model.add_field(dml::Field::RelationField(field));
                    };
                }
                RefinedRelationWalker::ImplicitManyToMany(relation) => {
                    for relation_field in [relation.field_a(), relation.field_b()] {
                        let ast_field = relation_field.ast_field();
                        let arity = self.lift_arity(&ast_field.arity);
                        let relation_info = dml::RelationInfo::new(relation_field.related_model().name());
                        let referential_arity = self.lift_arity(&relation_field.referential_arity());
                        let mut field =
                            dml::RelationField::new(relation_field.name(), arity, referential_arity, relation_info);

                        common_dml_fields(&mut field, relation_field);

                        let primary_key = relation_field.related_model().primary_key().unwrap();
                        field.relation_info.references =
                            primary_key.fields().map(|field| field.name().to_owned()).collect();

                        field.relation_info.fields = relation_field
                            .fields()
                            .into_iter()
                            .flatten()
                            .map(|f| f.name().to_owned())
                            .collect();

                        let model = schema.find_model_mut(relation_field.model().name());
                        model.add_field(dml::Field::RelationField(field));
                        field_ids_for_sorting.insert(
                            (relation_field.model().name(), relation_field.name()),
                            relation_field.field_id(),
                        );
                    }
                }
                RefinedRelationWalker::TwoWayEmbeddedManyToMany(relation) => {
                    for relation_field in [relation.field_a(), relation.field_b()] {
                        let ast_field = relation_field.ast_field();
                        let arity = self.lift_arity(&ast_field.arity);
                        let relation_info = dml::RelationInfo::new(relation_field.related_model().name());
                        let referential_arity = self.lift_arity(&relation_field.referential_arity());

                        let mut field =
                            dml::RelationField::new(relation_field.name(), arity, referential_arity, relation_info);

                        common_dml_fields(&mut field, relation_field);

                        field.relation_info.references = relation_field
                            .referenced_fields()
                            .into_iter()
                            .flatten()
                            .map(|f| f.name().to_owned())
                            .collect();

                        field.relation_info.fields = relation_field
                            .fields()
                            .into_iter()
                            .flatten()
                            .map(|f| f.name().to_owned())
                            .collect();

                        let model = schema.find_model_mut(relation_field.model().name());
                        model.add_field(dml::Field::RelationField(field));
                        field_ids_for_sorting.insert(
                            (relation_field.model().name(), relation_field.name()),
                            relation_field.field_id(),
                        );
                    }
                }
            }
        }
    }

    fn lift_composite_type(&self, walker: CompositeTypeWalker<'_>) -> CompositeType {
        let mut fields = Vec::new();

        for field in walker.fields() {
            let field = CompositeTypeField {
                name: field.name().to_owned(),
                r#type: self.lift_composite_type_field_type(field, field.r#type()),
                arity: self.lift_arity(&field.arity()),
                database_name: field.mapped_name().map(String::from),
                documentation: field.documentation().map(ToString::to_string),
                default_value: field.default_value().map(|value| dml::DefaultValue {
                    kind: dml_default_kind(value, field.r#type().as_builtin_scalar()),
                    db_name: None,
                }),
                is_commented_out: false,
            };

            fields.push(field);
        }

        CompositeType {
            name: walker.name().to_owned(),
            fields,
        }
    }

    /// Internal: Validates a model AST node and lifts it to a DML model.
    fn lift_model(
        &self,
        walker: ModelWalker<'a>,
        field_ids_for_sorting: &mut HashMap<(&'a str, &'a str), ast::FieldId>,
    ) -> dml::Model {
        let ast_model = walker.ast_model();
        let mut model = dml::Model::new(ast_model.name().to_owned(), None);

        model.documentation = ast_model.documentation().map(String::from);
        model.database_name = walker.mapped_name().map(String::from);
        model.is_ignored = walker.is_ignored();
        model.schema = walker.schema().map(|(s, _)| s.to_owned());

        model.primary_key = walker.primary_key().map(|pk| dml::PrimaryKeyDefinition {
            name: pk.name().map(String::from),
            db_name: pk.constraint_name(self.connector).map(|c| c.into_owned()),
            fields: pk
                .scalar_field_attributes()
                .map(|field|
                     //TODO (extended indexes) here it is ok to pass sort and length with out a preview flag
                     // check since this is coming from the ast and the parsing would reject the args without
                     // the flag set
                     // When we start using the extra args here could be a place to fill in the defaults.
                     PrimaryKeyField {
                         name: field.as_index_field().as_scalar_field().unwrap().name().to_owned(),
                         sort_order: field.sort_order().map(parser_database_sort_order_to_dml_sort_order),
                         length: field.length(),
                     })
                .collect(),
            defined_on_field: pk.is_defined_on_field(),
            // By default a primary key is always clustered in any database.
            clustered: pk.clustered(),
        });

        model.indices = walker
            .indexes()
            .map(|idx| {
                let fields = idx
                    .scalar_field_attributes()
                    .map(|field| {
                        let path = field
                            .as_path_to_indexed_field()
                            .into_iter()
                            .map(|(a, b)| (a.to_owned(), b.map(|s| s.to_owned())))
                            .collect();

                        let sort_order = field.sort_order().map(parser_database_sort_order_to_dml_sort_order);
                        let length = field.length();
                        let operator_class = field.operator_class().map(convert_op_class);

                        IndexField {
                            path,
                            sort_order,
                            length,
                            operator_class,
                        }
                    })
                    .collect();

                let tpe = match idx.index_type() {
                    db::IndexType::Unique => dml::IndexType::Unique,
                    db::IndexType::Normal => dml::IndexType::Normal,
                    db::IndexType::Fulltext => dml::IndexType::Fulltext,
                };

                let algorithm = idx.algorithm().map(|using| match using {
                    IndexAlgorithm::BTree => dml::IndexAlgorithm::BTree,
                    IndexAlgorithm::Hash => dml::IndexAlgorithm::Hash,
                    IndexAlgorithm::Gist => dml::IndexAlgorithm::Gist,
                    IndexAlgorithm::Gin => dml::IndexAlgorithm::Gin,
                    IndexAlgorithm::SpGist => dml::IndexAlgorithm::SpGist,
                    IndexAlgorithm::Brin => dml::IndexAlgorithm::Brin,
                });

                dml::IndexDefinition {
                    name: idx.name().map(String::from),
                    db_name: Some(idx.constraint_name(self.connector).into_owned()),
                    fields,
                    tpe,
                    algorithm,
                    defined_on_field: idx.is_defined_on_field(),
                    // By default an index that is not a primary key is always non-clustered in any database.
                    clustered: idx.clustered(),
                }
            })
            .collect();

        for scalar_field in walker.scalar_fields() {
            let field_id = scalar_field.field_id();
            let ast_field = &ast_model[field_id];
            let arity = self.lift_arity(&ast_field.arity);

            field_ids_for_sorting.insert((ast_model.name(), ast_field.name()), field_id);

            let field_type = match &scalar_field.scalar_field_type() {
                db::ScalarFieldType::CompositeType(ctid) => {
                    let mut field = dml::CompositeField::new();
                    field.name = scalar_field.name().to_owned();
                    field.composite_type = self.db.ast()[*ctid].name().to_owned();
                    field.documentation = ast_field.documentation().map(String::from);
                    field.is_ignored = scalar_field.is_ignored();
                    field.database_name = scalar_field.mapped_name().map(String::from);
                    field.arity = arity;

                    model.add_field(dml::Field::CompositeField(field));
                    continue;
                }
                _ => self.lift_scalar_field_type(ast_field, &scalar_field.scalar_field_type(), scalar_field),
            };

            let mut field = dml::ScalarField::new(ast_field.name(), arity, field_type);

            field.documentation = ast_field.documentation().map(String::from);
            field.is_ignored = scalar_field.is_ignored();
            field.is_updated_at = scalar_field.is_updated_at();
            field.database_name = scalar_field.mapped_name().map(String::from);
            field.default_value = scalar_field.default_value().map(|d| dml::DefaultValue {
                kind: dml_default_kind(d.value(), scalar_field.scalar_type()),
                db_name: Some(d.constraint_name(self.connector).into())
                    .filter(|_| self.connector.supports_named_default_values()),
            });

            model.add_field(dml::Field::ScalarField(field));
        }

        model
    }

    /// Internal: Validates an enum AST node.
    fn lift_enum(&self, r#enum: EnumWalker<'_>) -> dml::Enum {
        let mut en = dml::Enum::new(r#enum.name(), vec![]);

        for value in r#enum.values() {
            en.add_value(self.lift_enum_value(value));
        }

        en.documentation = r#enum.ast_enum().documentation().map(String::from);
        en.database_name = r#enum.mapped_name().map(String::from);
        en.schema = r#enum.schema().map(|(s, _)| s.to_owned());
        en
    }

    /// Internal: Lifts an enum value AST node.
    fn lift_enum_value(&self, value: EnumValueWalker<'_>) -> dml::EnumValue {
        let mut enum_value = dml::EnumValue::new(value.name());
        enum_value.documentation = value.documentation().map(String::from);
        enum_value.database_name = value.mapped_name().map(String::from);
        enum_value
    }

    /// Internal: Lift a field's arity.
    fn lift_arity(&self, field_arity: &ast::FieldArity) -> dml::FieldArity {
        match field_arity {
            ast::FieldArity::Required => dml::FieldArity::Required,
            ast::FieldArity::Optional => dml::FieldArity::Optional,
            ast::FieldArity::List => dml::FieldArity::List,
        }
    }

    fn lift_scalar_field_type(
        &self,
        ast_field: &ast::Field,
        scalar_field_type: &db::ScalarFieldType,
        scalar_field: ScalarFieldWalker<'_>,
    ) -> dml::FieldType {
        match scalar_field_type {
            db::ScalarFieldType::CompositeType(_) => {
                unreachable!();
            }
            db::ScalarFieldType::Enum(enum_id) => {
                let enum_name = &self.db.ast()[*enum_id].name.name;
                dml::FieldType::Enum(enum_name.to_owned())
            }
            db::ScalarFieldType::Unsupported(_) => {
                dml::FieldType::Unsupported(ast_field.field_type.as_unsupported().unwrap().0.to_owned())
            }
            db::ScalarFieldType::BuiltInScalar(scalar_type) => {
                let native_type = scalar_field.raw_native_type().map(|(_, name, args, _)| {
                    self.connector
                        .parse_native_type(name, args.to_owned(), scalar_field.ast_field().span())
                        .unwrap()
                });
                dml::FieldType::Scalar(
                    parser_database_scalar_type_to_dml_scalar_type(*scalar_type),
                    native_type.map(datamodel_connector_native_type_to_dml_native_type),
                )
            }
        }
    }

    fn lift_composite_type_field_type(
        &self,
        composite_type_field: CompositeTypeFieldWalker<'_>,
        scalar_field_type: &db::ScalarFieldType,
    ) -> CompositeTypeFieldType {
        match scalar_field_type {
            db::ScalarFieldType::CompositeType(ctid) => {
                CompositeTypeFieldType::CompositeType(self.db.ast()[*ctid].name().to_owned())
            }
            db::ScalarFieldType::BuiltInScalar(scalar_type) => {
                let native_type = composite_type_field.raw_native_type().map(|(_, name, args, _)| {
                    self.connector
                        .parse_native_type(name, args.to_owned(), composite_type_field.ast_field().span())
                        .unwrap()
                });

                CompositeTypeFieldType::Scalar(
                    parser_database_scalar_type_to_dml_scalar_type(*scalar_type),
                    native_type.map(datamodel_connector_native_type_to_dml_native_type),
                )
            }
            db::ScalarFieldType::Enum(enum_id) => {
                let enum_name = &self.db.ast()[*enum_id].name.name;

                CompositeTypeFieldType::Enum(enum_name.to_owned())
            }
            db::ScalarFieldType::Unsupported(_) => {
                let field = composite_type_field
                    .ast_field()
                    .field_type
                    .as_unsupported()
                    .unwrap()
                    .0
                    .to_owned();

                CompositeTypeFieldType::Unsupported(field)
            }
        }
    }
}

fn parser_database_sort_order_to_dml_sort_order(sort_order: parser_database::SortOrder) -> dml::SortOrder {
    match sort_order {
        parser_database::SortOrder::Asc => dml::SortOrder::Asc,
        parser_database::SortOrder::Desc => dml::SortOrder::Desc,
    }
}

fn parser_database_referential_action_to_dml_referential_action(
    ra: parser_database::ReferentialAction,
) -> dml::ReferentialAction {
    match ra {
        parser_database::ReferentialAction::Cascade => dml::ReferentialAction::Cascade,
        parser_database::ReferentialAction::SetNull => dml::ReferentialAction::SetNull,
        parser_database::ReferentialAction::SetDefault => dml::ReferentialAction::SetDefault,
        parser_database::ReferentialAction::Restrict => dml::ReferentialAction::Restrict,
        parser_database::ReferentialAction::NoAction => dml::ReferentialAction::NoAction,
    }
}

fn parser_database_scalar_type_to_dml_scalar_type(st: parser_database::ScalarType) -> dml::ScalarType {
    st.as_str().parse().unwrap()
}

fn datamodel_connector_native_type_to_dml_native_type(
    instance: datamodel_connector::NativeTypeInstance,
) -> dml::NativeTypeInstance {
    dml::NativeTypeInstance {
        name: instance.name,
        args: instance.args,
        serialized_native_type: instance.serialized_native_type,
    }
}

fn dml_default_kind(default_value: &ast::Expression, scalar_type: Option<ScalarType>) -> dml::DefaultKind {
    // This has all been validated in parser-database, so unwrapping is always safe.
    match default_value {
        ast::Expression::Function(funcname, args, _) if funcname == "dbgenerated" => {
            DefaultKind::Expression(ValueGenerator::new_dbgenerated(
                args.arguments
                    .get(0)
                    .and_then(|arg| arg.value.as_string_value())
                    .map(|(val, _)| val.to_owned())
                    .unwrap_or_else(String::new),
            ))
        }
        ast::Expression::Function(funcname, _, _) if funcname == "auto" => {
            DefaultKind::Expression(ValueGenerator::new_auto())
        }
        ast::Expression::Function(funcname, _args, _) if funcname == "autoincrement" => {
            DefaultKind::Expression(ValueGenerator::new_autoincrement())
        }
        ast::Expression::Function(funcname, _args, _) if funcname == "sequence" => {
            DefaultKind::Expression(ValueGenerator::new_sequence(Vec::new()))
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
        ast::Expression::NumericValue(num, _) => match scalar_type {
            Some(ScalarType::Int) => DefaultKind::Single(PrismaValue::Int(num.parse().unwrap())),
            Some(ScalarType::BigInt) => DefaultKind::Single(PrismaValue::BigInt(num.parse().unwrap())),
            Some(ScalarType::Float) => DefaultKind::Single(PrismaValue::Float(num.parse().unwrap())),
            Some(ScalarType::Decimal) => DefaultKind::Single(PrismaValue::Float(num.parse().unwrap())),
            other => unreachable!("{:?}", other),
        },
        ast::Expression::ConstantValue(v, _) => match scalar_type {
            Some(ScalarType::Boolean) => DefaultKind::Single(PrismaValue::Boolean(v.parse().unwrap())),
            None => DefaultKind::Single(PrismaValue::Enum(v.to_owned())),
            other => unreachable!("{:?}", other),
        },
        ast::Expression::StringValue(v, _) => match scalar_type {
            Some(ScalarType::DateTime) => DefaultKind::Single(PrismaValue::DateTime(v.parse().unwrap())),
            Some(ScalarType::String) => DefaultKind::Single(PrismaValue::String(v.parse().unwrap())),
            Some(ScalarType::Json) => DefaultKind::Single(PrismaValue::Json(v.parse().unwrap())),
            Some(ScalarType::Decimal) => DefaultKind::Single(PrismaValue::Float(v.parse().unwrap())),
            Some(ScalarType::Bytes) => {
                DefaultKind::Single(PrismaValue::Bytes(::dml::prisma_value::decode_bytes(v).unwrap()))
            }
            other => unreachable!("{:?}", other),
        },
        ast::Expression::Array(values, _) => {
            let values = values
                .iter()
                .map(|expr| dml_default_kind(expr, scalar_type).unwrap_single())
                .collect();

            DefaultKind::Single(PrismaValue::List(values))
        }
        other => unreachable!("{:?}", other),
    }
}

fn convert_op_class(from_db: OperatorClassWalker<'_>) -> dml::OperatorClass {
    match from_db.get() {
        // gist
        Either::Left(db::OperatorClass::InetOps) => dml::OperatorClass::InetOps,

        // gin
        Either::Left(db::OperatorClass::JsonbOps) => dml::OperatorClass::JsonbOps,
        Either::Left(db::OperatorClass::JsonbPathOps) => dml::OperatorClass::JsonbPathOps,
        Either::Left(db::OperatorClass::ArrayOps) => dml::OperatorClass::ArrayOps,

        // sp-gist
        Either::Left(db::OperatorClass::TextOps) => dml::OperatorClass::TextOps,

        // brin
        Either::Left(db::OperatorClass::BitMinMaxOps) => dml::OperatorClass::BitMinMaxOps,
        Either::Left(db::OperatorClass::VarBitMinMaxOps) => dml::OperatorClass::VarBitMinMaxOps,
        Either::Left(db::OperatorClass::BpcharBloomOps) => dml::OperatorClass::BpcharBloomOps,
        Either::Left(db::OperatorClass::BpcharMinMaxOps) => dml::OperatorClass::BpcharMinMaxOps,
        Either::Left(db::OperatorClass::ByteaBloomOps) => dml::OperatorClass::ByteaBloomOps,
        Either::Left(db::OperatorClass::ByteaMinMaxOps) => dml::OperatorClass::ByteaMinMaxOps,
        Either::Left(db::OperatorClass::DateBloomOps) => dml::OperatorClass::DateBloomOps,
        Either::Left(db::OperatorClass::DateMinMaxOps) => dml::OperatorClass::DateMinMaxOps,
        Either::Left(db::OperatorClass::DateMinMaxMultiOps) => dml::OperatorClass::DateMinMaxMultiOps,
        Either::Left(db::OperatorClass::Float4BloomOps) => dml::OperatorClass::Float4BloomOps,
        Either::Left(db::OperatorClass::Float4MinMaxOps) => dml::OperatorClass::Float4MinMaxOps,
        Either::Left(db::OperatorClass::Float4MinMaxMultiOps) => dml::OperatorClass::Float4MinMaxMultiOps,
        Either::Left(db::OperatorClass::Float8BloomOps) => dml::OperatorClass::Float8BloomOps,
        Either::Left(db::OperatorClass::Float8MinMaxOps) => dml::OperatorClass::Float8MinMaxOps,
        Either::Left(db::OperatorClass::Float8MinMaxMultiOps) => dml::OperatorClass::Float8MinMaxMultiOps,
        Either::Left(db::OperatorClass::InetInclusionOps) => dml::OperatorClass::InetInclusionOps,
        Either::Left(db::OperatorClass::InetBloomOps) => dml::OperatorClass::InetBloomOps,
        Either::Left(db::OperatorClass::InetMinMaxOps) => dml::OperatorClass::InetMinMaxOps,
        Either::Left(db::OperatorClass::InetMinMaxMultiOps) => dml::OperatorClass::InetMinMaxMultiOps,
        Either::Left(db::OperatorClass::Int2BloomOps) => dml::OperatorClass::Int2BloomOps,
        Either::Left(db::OperatorClass::Int2MinMaxOps) => dml::OperatorClass::Int2MinMaxOps,
        Either::Left(db::OperatorClass::Int2MinMaxMultiOps) => dml::OperatorClass::Int2MinMaxMultiOps,
        Either::Left(db::OperatorClass::Int4BloomOps) => dml::OperatorClass::Int4BloomOps,
        Either::Left(db::OperatorClass::Int4MinMaxOps) => dml::OperatorClass::Int4MinMaxOps,
        Either::Left(db::OperatorClass::Int4MinMaxMultiOps) => dml::OperatorClass::Int4MinMaxMultiOps,
        Either::Left(db::OperatorClass::Int8BloomOps) => dml::OperatorClass::Int8BloomOps,
        Either::Left(db::OperatorClass::Int8MinMaxOps) => dml::OperatorClass::Int8MinMaxOps,
        Either::Left(db::OperatorClass::Int8MinMaxMultiOps) => dml::OperatorClass::Int8MinMaxMultiOps,
        Either::Left(db::OperatorClass::NumericBloomOps) => dml::OperatorClass::NumericBloomOps,
        Either::Left(db::OperatorClass::NumericMinMaxOps) => dml::OperatorClass::NumericMinMaxOps,
        Either::Left(db::OperatorClass::NumericMinMaxMultiOps) => dml::OperatorClass::NumericMinMaxMultiOps,
        Either::Left(db::OperatorClass::OidBloomOps) => dml::OperatorClass::OidBloomOps,
        Either::Left(db::OperatorClass::OidMinMaxOps) => dml::OperatorClass::OidMinMaxOps,
        Either::Left(db::OperatorClass::OidMinMaxMultiOps) => dml::OperatorClass::OidMinMaxMultiOps,
        Either::Left(db::OperatorClass::TextBloomOps) => dml::OperatorClass::TextBloomOps,
        Either::Left(db::OperatorClass::TextMinMaxOps) => dml::OperatorClass::TextMinMaxOps,
        Either::Left(db::OperatorClass::TimestampBloomOps) => dml::OperatorClass::TimestampBloomOps,
        Either::Left(db::OperatorClass::TimestampMinMaxOps) => dml::OperatorClass::TimestampMinMaxOps,
        Either::Left(db::OperatorClass::TimestampMinMaxMultiOps) => dml::OperatorClass::TimestampMinMaxMultiOps,
        Either::Left(db::OperatorClass::TimestampTzBloomOps) => dml::OperatorClass::TimestampTzBloomOps,
        Either::Left(db::OperatorClass::TimestampTzMinMaxOps) => dml::OperatorClass::TimestampTzMinMaxOps,
        Either::Left(db::OperatorClass::TimestampTzMinMaxMultiOps) => dml::OperatorClass::TimestampTzMinMaxMultiOps,
        Either::Left(db::OperatorClass::TimeBloomOps) => dml::OperatorClass::TimeBloomOps,
        Either::Left(db::OperatorClass::TimeMinMaxOps) => dml::OperatorClass::TimeMinMaxOps,
        Either::Left(db::OperatorClass::TimeMinMaxMultiOps) => dml::OperatorClass::TimeMinMaxMultiOps,
        Either::Left(db::OperatorClass::TimeTzBloomOps) => dml::OperatorClass::TimeTzBloomOps,
        Either::Left(db::OperatorClass::TimeTzMinMaxOps) => dml::OperatorClass::TimeTzMinMaxOps,
        Either::Left(db::OperatorClass::TimeTzMinMaxMultiOps) => dml::OperatorClass::TimeTzMinMaxMultiOps,
        Either::Left(db::OperatorClass::UuidBloomOps) => dml::OperatorClass::UuidBloomOps,
        Either::Left(db::OperatorClass::UuidMinMaxOps) => dml::OperatorClass::UuidMinMaxOps,
        Either::Left(db::OperatorClass::UuidMinMaxMultiOps) => dml::OperatorClass::UuidMinMaxMultiOps,

        Either::Right(raw) => dml::OperatorClass::Raw(raw.to_string().into()),
    }
}
