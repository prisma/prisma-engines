use crate::{self as dml, *};
use either::Either;
use psl_core::{
    datamodel_connector::{constraint_names::ConstraintNames, walker_ext_traits::*, Connector, RelationMode},
    parser_database::{
        self as db,
        ast::{self, WithDocumentation, WithName, WithSpan},
        walkers::*,
        IndexAlgorithm, ScalarType,
    },
};
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
    relation_mode: RelationMode,
}

impl<'a> LiftAstToDml<'a> {
    pub(crate) fn new(
        db: &'a db::ParserDatabase,
        connector: &'static dyn Connector,
        relation_mode: RelationMode,
    ) -> LiftAstToDml<'a> {
        LiftAstToDml {
            db,
            connector,
            relation_mode,
        }
    }

    pub(crate) fn lift(&self) -> Datamodel {
        let mut schema = Datamodel::new();

        // We iterate over scalar fields, then relations, but we want the
        // order of fields in the Model to match the order of the fields in
        // the AST, so we need this bit of extra bookkeeping.
        //
        // (model_idx, field_name) -> sort_key
        let mut field_ids_for_sorting: HashMap<(&str, &str), ast::FieldId> = HashMap::new();

        for model in self.db.walk_models() {
            schema.models.push(self.lift_model(model, &mut field_ids_for_sorting));
        }

        for composite_type in self.db.walk_composite_types() {
            schema.composite_types.push(self.lift_composite_type(composite_type))
        }

        for r#enum in self.db.walk_enums() {
            schema.enums.push(self.lift_enum(r#enum))
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
        schema: &mut Datamodel,
        field_ids_for_sorting: &mut HashMap<(&'a str, &'a str), ast::FieldId>,
    ) {
        let active_connector = self.connector;
        let relation_mode = self.relation_mode;
        let common_dml_fields = |field: &mut RelationField, relation_field: RelationFieldWalker<'_>| {
            let ast_field = relation_field.ast_field();
            field.relation_info.on_delete = relation_field.explicit_on_delete().map(From::from);
            field.relation_info.on_update = relation_field.explicit_on_update().map(From::from);
            field.relation_info.name = relation_field.relation_name().to_string();
            field.documentation = ast_field.documentation().map(String::from);
            field.is_ignored = relation_field.is_ignored();
            field.supports_restrict_action(
                active_connector.supports_referential_action(&relation_mode, db::ReferentialAction::Restrict),
            );
            field.emulates_referential_actions(relation_mode.is_prisma());
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

                        let referenced_model = relation.referenced_model();

                        let relation_info = RelationInfo::new(referenced_model.name());

                        let forward_field_walker = relation.forward_relation_field().unwrap();
                        // Construct a relation field in the DML for an existing relation field in the source.
                        let arity = forward_field_walker.ast_field().arity.into();
                        let referential_arity = forward_field_walker.referential_arity().into();
                        let mut relation_field =
                            RelationField::new(forward_field_walker.name(), arity, referential_arity, relation_info);

                        let column_names: Vec<&str> = relation
                            .referencing_fields()
                            .unwrap()
                            .map(|f| f.database_name())
                            .collect();

                        let fk_name = relation.constraint_name(active_connector);

                        let default_name = ConstraintNames::foreign_key_constraint_name(
                            relation.referencing_model().database_name(),
                            &column_names,
                            self.connector,
                        );

                        let fk_name = if fk_name.as_ref() == default_name.as_str() {
                            None
                        } else {
                            Some(relation.constraint_name(active_connector).to_string())
                        };

                        relation_field.relation_info.fk_name = fk_name;

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
                        model.add_field(Field::RelationField(relation_field));
                    };

                    // Back field
                    {
                        let relation_info = RelationInfo::new(relation.referencing_model().name());
                        let model = schema.find_model_mut(relation.referenced_model().name());

                        let mut field = if let Some(relation_field) = relation.back_relation_field() {
                            let ast_field = relation_field.ast_field();
                            let arity = ast_field.arity.into();
                            let referential_arity = relation_field.referential_arity().into();
                            let mut field =
                                RelationField::new(relation_field.name(), arity, referential_arity, relation_info);

                            common_dml_fields(&mut field, relation_field);

                            field_ids_for_sorting.insert(
                                (relation_field.model().name(), relation_field.name()),
                                relation_field.field_id(),
                            );

                            field
                        } else {
                            // This is part of reformatting.
                            let arity = FieldArity::List;
                            let referential_arity = FieldArity::List;
                            let mut field = RelationField::new(
                                relation.referencing_model().name(),
                                arity,
                                referential_arity,
                                relation_info,
                            );
                            field.is_ignored = relation.referencing_model().is_ignored();
                            field
                        };

                        field.relation_info.name = relation.relation_name().to_string();
                        model.add_field(Field::RelationField(field));
                    };
                }
                RefinedRelationWalker::ImplicitManyToMany(relation) => {
                    for relation_field in [relation.field_a(), relation.field_b()] {
                        let ast_field = relation_field.ast_field();
                        let arity = ast_field.arity.into();
                        let relation_info = RelationInfo::new(relation_field.related_model().name());
                        let referential_arity = relation_field.referential_arity().into();
                        let mut field =
                            RelationField::new(relation_field.name(), arity, referential_arity, relation_info);

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
                        model.add_field(Field::RelationField(field));
                        field_ids_for_sorting.insert(
                            (relation_field.model().name(), relation_field.name()),
                            relation_field.field_id(),
                        );
                    }
                }
                RefinedRelationWalker::TwoWayEmbeddedManyToMany(relation) => {
                    for relation_field in [relation.field_a(), relation.field_b()] {
                        let ast_field = relation_field.ast_field();
                        let arity = ast_field.arity.into();
                        let relation_info = RelationInfo::new(relation_field.related_model().name());
                        let referential_arity = relation_field.referential_arity().into();

                        let mut field =
                            RelationField::new(relation_field.name(), arity, referential_arity, relation_info);

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
                        model.add_field(Field::RelationField(field));
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
                arity: field.arity().into(),
                database_name: field.mapped_name().map(String::from),
                documentation: field.documentation().map(ToString::to_string),
                default_value: field.default_value().map(|value| DefaultValue {
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
    ) -> Model {
        let ast_model = walker.ast_model();
        let mut model = Model::new(ast_model.name().to_owned(), None);

        model.documentation = ast_model.documentation().map(String::from);
        model.database_name = walker.mapped_name().map(String::from);
        model.is_ignored = walker.is_ignored();
        model.schema = walker.schema().map(|(s, _)| s.to_owned());
        model.is_view = walker.ast_model().is_view();

        model.primary_key = walker.primary_key().map(|pk| PrimaryKeyDefinition {
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

                let algorithm = idx.algorithm().map(|using| match using {
                    IndexAlgorithm::BTree => model::IndexAlgorithm::BTree,
                    IndexAlgorithm::Hash => model::IndexAlgorithm::Hash,
                    IndexAlgorithm::Gist => model::IndexAlgorithm::Gist,
                    IndexAlgorithm::Gin => model::IndexAlgorithm::Gin,
                    IndexAlgorithm::SpGist => model::IndexAlgorithm::SpGist,
                    IndexAlgorithm::Brin => model::IndexAlgorithm::Brin,
                });

                IndexDefinition {
                    name: idx.name().map(String::from),
                    db_name: Some(idx.constraint_name(self.connector).into_owned()),
                    fields,
                    tpe: idx.index_type(),
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

            field_ids_for_sorting.insert((ast_model.name(), ast_field.name()), field_id);

            let field_type = match &scalar_field.scalar_field_type() {
                db::ScalarFieldType::CompositeType(ctid) => {
                    let mut field = CompositeField::new();
                    field.name = scalar_field.name().to_owned();
                    field.composite_type = self.db.ast()[*ctid].name().to_owned();
                    field.documentation = ast_field.documentation().map(String::from);
                    field.is_ignored = scalar_field.is_ignored();
                    field.database_name = scalar_field.mapped_name().map(String::from);
                    field.arity = ast_field.arity.into();

                    model.add_field(Field::CompositeField(field));
                    continue;
                }
                _ => self.lift_scalar_field_type(ast_field, &scalar_field.scalar_field_type(), scalar_field),
            };

            let mut field = ScalarField::new(ast_field.name(), ast_field.arity.into(), field_type);

            field.documentation = ast_field.documentation().map(String::from);
            field.is_ignored = scalar_field.is_ignored();
            field.is_updated_at = scalar_field.is_updated_at();
            field.database_name = scalar_field.mapped_name().map(String::from);
            field.default_value = scalar_field.default_value().map(|d| DefaultValue {
                kind: dml_default_kind(d.value(), scalar_field.scalar_type()),
                db_name: Some(d.constraint_name(self.connector).into())
                    .filter(|_| self.connector.supports_named_default_values()),
            });

            model.add_field(Field::ScalarField(field));
        }

        model
    }

    /// Internal: Validates an enum AST node.
    fn lift_enum(&self, r#enum: EnumWalker<'_>) -> Enum {
        let mut en = Enum::new(r#enum.name(), vec![], None);

        for value in r#enum.values() {
            en.add_value(self.lift_enum_value(value));
        }

        en.documentation = r#enum.ast_enum().documentation().map(String::from);
        en.database_name = r#enum.mapped_name().map(String::from);
        en.schema = r#enum.schema().map(|(s, _)| s.to_owned());
        en
    }

    /// Internal: Lifts an enum value AST node.
    fn lift_enum_value(&self, value: EnumValueWalker<'_>) -> EnumValue {
        let mut enum_value = EnumValue::new(value.name());
        enum_value.documentation = value.documentation().map(String::from);
        enum_value.database_name = value.mapped_name().map(String::from);
        enum_value
    }

    fn lift_scalar_field_type(
        &self,
        ast_field: &ast::Field,
        scalar_field_type: &db::ScalarFieldType,
        scalar_field: ScalarFieldWalker<'_>,
    ) -> FieldType {
        match scalar_field_type {
            db::ScalarFieldType::CompositeType(_) => {
                unreachable!();
            }
            db::ScalarFieldType::Enum(enum_id) => {
                let enum_name = &self.db.ast()[*enum_id].name.name;
                FieldType::Enum(enum_name.to_owned())
            }
            db::ScalarFieldType::Unsupported(_) => {
                FieldType::Unsupported(ast_field.field_type.as_unsupported().unwrap().0.to_owned())
            }
            db::ScalarFieldType::BuiltInScalar(scalar_type) => {
                let native_type = scalar_field.raw_native_type().map(|(_, name, args, _)| {
                    self.connector
                        .parse_native_type(name, args, scalar_field.ast_field().span(), &mut Default::default())
                        .unwrap()
                });
                FieldType::Scalar(
                    parser_database_scalar_type_to_dml_scalar_type(*scalar_type),
                    native_type.map(|nt| dml::NativeTypeInstance {
                        native_type: nt,
                        connector: self.connector,
                    }),
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
                        .parse_native_type(
                            name,
                            args,
                            composite_type_field.ast_field().span(),
                            &mut Default::default(),
                        )
                        .unwrap()
                });

                CompositeTypeFieldType::Scalar(
                    parser_database_scalar_type_to_dml_scalar_type(*scalar_type),
                    native_type.map(|nt| dml::NativeTypeInstance {
                        native_type: nt,
                        connector: self.connector,
                    }),
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

fn parser_database_sort_order_to_dml_sort_order(sort_order: db::SortOrder) -> SortOrder {
    match sort_order {
        db::SortOrder::Asc => SortOrder::Asc,
        db::SortOrder::Desc => SortOrder::Desc,
    }
}

fn parser_database_scalar_type_to_dml_scalar_type(st: db::ScalarType) -> dml::ScalarType {
    st.as_str().parse().unwrap()
}

fn dml_default_kind(default_value: &ast::Expression, scalar_type: Option<ScalarType>) -> DefaultKind {
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
            Some(ScalarType::Bytes) => DefaultKind::Single(PrismaValue::Bytes(prisma_value::decode_bytes(v).unwrap())),
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

fn convert_op_class(from_db: OperatorClassWalker<'_>) -> OperatorClass {
    match from_db.get() {
        // gist
        Either::Left(db::OperatorClass::InetOps) => OperatorClass::InetOps,

        // gin
        Either::Left(db::OperatorClass::JsonbOps) => OperatorClass::JsonbOps,
        Either::Left(db::OperatorClass::JsonbPathOps) => OperatorClass::JsonbPathOps,
        Either::Left(db::OperatorClass::ArrayOps) => OperatorClass::ArrayOps,

        // sp-gist
        Either::Left(db::OperatorClass::TextOps) => OperatorClass::TextOps,

        // brin
        Either::Left(db::OperatorClass::BitMinMaxOps) => OperatorClass::BitMinMaxOps,
        Either::Left(db::OperatorClass::VarBitMinMaxOps) => OperatorClass::VarBitMinMaxOps,
        Either::Left(db::OperatorClass::BpcharBloomOps) => OperatorClass::BpcharBloomOps,
        Either::Left(db::OperatorClass::BpcharMinMaxOps) => OperatorClass::BpcharMinMaxOps,
        Either::Left(db::OperatorClass::ByteaBloomOps) => OperatorClass::ByteaBloomOps,
        Either::Left(db::OperatorClass::ByteaMinMaxOps) => OperatorClass::ByteaMinMaxOps,
        Either::Left(db::OperatorClass::DateBloomOps) => OperatorClass::DateBloomOps,
        Either::Left(db::OperatorClass::DateMinMaxOps) => OperatorClass::DateMinMaxOps,
        Either::Left(db::OperatorClass::DateMinMaxMultiOps) => OperatorClass::DateMinMaxMultiOps,
        Either::Left(db::OperatorClass::Float4BloomOps) => OperatorClass::Float4BloomOps,
        Either::Left(db::OperatorClass::Float4MinMaxOps) => OperatorClass::Float4MinMaxOps,
        Either::Left(db::OperatorClass::Float4MinMaxMultiOps) => OperatorClass::Float4MinMaxMultiOps,
        Either::Left(db::OperatorClass::Float8BloomOps) => OperatorClass::Float8BloomOps,
        Either::Left(db::OperatorClass::Float8MinMaxOps) => OperatorClass::Float8MinMaxOps,
        Either::Left(db::OperatorClass::Float8MinMaxMultiOps) => OperatorClass::Float8MinMaxMultiOps,
        Either::Left(db::OperatorClass::InetInclusionOps) => OperatorClass::InetInclusionOps,
        Either::Left(db::OperatorClass::InetBloomOps) => OperatorClass::InetBloomOps,
        Either::Left(db::OperatorClass::InetMinMaxOps) => OperatorClass::InetMinMaxOps,
        Either::Left(db::OperatorClass::InetMinMaxMultiOps) => OperatorClass::InetMinMaxMultiOps,
        Either::Left(db::OperatorClass::Int2BloomOps) => OperatorClass::Int2BloomOps,
        Either::Left(db::OperatorClass::Int2MinMaxOps) => OperatorClass::Int2MinMaxOps,
        Either::Left(db::OperatorClass::Int2MinMaxMultiOps) => OperatorClass::Int2MinMaxMultiOps,
        Either::Left(db::OperatorClass::Int4BloomOps) => OperatorClass::Int4BloomOps,
        Either::Left(db::OperatorClass::Int4MinMaxOps) => OperatorClass::Int4MinMaxOps,
        Either::Left(db::OperatorClass::Int4MinMaxMultiOps) => OperatorClass::Int4MinMaxMultiOps,
        Either::Left(db::OperatorClass::Int8BloomOps) => OperatorClass::Int8BloomOps,
        Either::Left(db::OperatorClass::Int8MinMaxOps) => OperatorClass::Int8MinMaxOps,
        Either::Left(db::OperatorClass::Int8MinMaxMultiOps) => OperatorClass::Int8MinMaxMultiOps,
        Either::Left(db::OperatorClass::NumericBloomOps) => OperatorClass::NumericBloomOps,
        Either::Left(db::OperatorClass::NumericMinMaxOps) => OperatorClass::NumericMinMaxOps,
        Either::Left(db::OperatorClass::NumericMinMaxMultiOps) => OperatorClass::NumericMinMaxMultiOps,
        Either::Left(db::OperatorClass::OidBloomOps) => OperatorClass::OidBloomOps,
        Either::Left(db::OperatorClass::OidMinMaxOps) => OperatorClass::OidMinMaxOps,
        Either::Left(db::OperatorClass::OidMinMaxMultiOps) => OperatorClass::OidMinMaxMultiOps,
        Either::Left(db::OperatorClass::TextBloomOps) => OperatorClass::TextBloomOps,
        Either::Left(db::OperatorClass::TextMinMaxOps) => OperatorClass::TextMinMaxOps,
        Either::Left(db::OperatorClass::TimestampBloomOps) => OperatorClass::TimestampBloomOps,
        Either::Left(db::OperatorClass::TimestampMinMaxOps) => OperatorClass::TimestampMinMaxOps,
        Either::Left(db::OperatorClass::TimestampMinMaxMultiOps) => OperatorClass::TimestampMinMaxMultiOps,
        Either::Left(db::OperatorClass::TimestampTzBloomOps) => OperatorClass::TimestampTzBloomOps,
        Either::Left(db::OperatorClass::TimestampTzMinMaxOps) => OperatorClass::TimestampTzMinMaxOps,
        Either::Left(db::OperatorClass::TimestampTzMinMaxMultiOps) => OperatorClass::TimestampTzMinMaxMultiOps,
        Either::Left(db::OperatorClass::TimeBloomOps) => OperatorClass::TimeBloomOps,
        Either::Left(db::OperatorClass::TimeMinMaxOps) => OperatorClass::TimeMinMaxOps,
        Either::Left(db::OperatorClass::TimeMinMaxMultiOps) => OperatorClass::TimeMinMaxMultiOps,
        Either::Left(db::OperatorClass::TimeTzBloomOps) => OperatorClass::TimeTzBloomOps,
        Either::Left(db::OperatorClass::TimeTzMinMaxOps) => OperatorClass::TimeTzMinMaxOps,
        Either::Left(db::OperatorClass::TimeTzMinMaxMultiOps) => OperatorClass::TimeTzMinMaxMultiOps,
        Either::Left(db::OperatorClass::UuidBloomOps) => OperatorClass::UuidBloomOps,
        Either::Left(db::OperatorClass::UuidMinMaxOps) => OperatorClass::UuidMinMaxOps,
        Either::Left(db::OperatorClass::UuidMinMaxMultiOps) => OperatorClass::UuidMinMaxMultiOps,

        Either::Right(raw) => OperatorClass::Raw(raw.to_string().into()),
    }
}
