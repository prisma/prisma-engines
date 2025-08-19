use crate::{
    binding,
    result_node::{ResultNode, ResultNodeBuilder},
};
use bon::builder;
use indexmap::IndexSet;
use itertools::Itertools;
use psl::datamodel_connector::Flavour;
use query_core::{
    CreateManyRecordsFields, DeleteRecordFields, Node, Query, QueryGraph, ReadQuery, UpdateManyRecordsFields,
    UpdateRecord, WriteQuery, schema::constants::aggregations,
};
use query_structure::{
    AggregationSelection, FieldArity, FieldSelection, FieldTypeInformation, ScalarField, SelectedField, Type,
    TypeIdentifier,
};
use serde::Serialize;
use std::{borrow::Cow, collections::HashMap, fmt};

pub fn map_result_structure(graph: &QueryGraph, builder: &mut ResultNodeBuilder) -> Option<ResultNode> {
    graph
        .result_nodes()
        .chain(graph.leaf_nodes())
        .find_map(|idx| {
            if let Node::Query(query) = graph.node_content(&idx)? {
                map_query(query, builder)
            } else {
                None
            }
        })
        .or_else(|| {
            graph
                .result_nodes()
                .chain(graph.leaf_nodes())
                .all(|idx| match graph.node_content(&idx) {
                    Some(Node::Query(Query::Write(WriteQuery::QueryRaw(_) | WriteQuery::ExecuteRaw(_)))) => false,
                    Some(Node::Query(Query::Write(write))) => write.returns().is_none(),
                    _ => false,
                })
                .then_some(ResultNode::AffectedRows)
        })
}

fn map_query(query: &Query, builder: &mut ResultNodeBuilder) -> Option<ResultNode> {
    match query {
        Query::Read(read_query) => map_read_query(read_query, builder, None),
        Query::Write(write_query) => map_write_query(write_query, builder),
    }
}

fn map_read_query(
    query: &ReadQuery,
    builder: &mut ResultNodeBuilder,
    object_name: Option<Cow<'static, str>>,
) -> Option<ResultNode> {
    match query {
        ReadQuery::RecordQuery(q) => get_result_node()
            .field_selection(&q.selected_fields)
            .selection_order(&q.selection_order)
            .nested_queries(&q.nested)
            .builder(builder)
            .maybe_original_name(object_name)
            .uses_relation_joins(q.relation_load_strategy.is_join())
            .call(),
        ReadQuery::ManyRecordsQuery(q) => get_result_node()
            .field_selection(&q.selected_fields)
            .selection_order(&q.selection_order)
            .nested_queries(&q.nested)
            .builder(builder)
            .maybe_original_name(object_name)
            .uses_relation_joins(q.relation_load_strategy.is_join())
            .call(),
        ReadQuery::RelatedRecordsQuery(q) => get_result_node()
            .field_selection(&q.selected_fields)
            .selection_order(&q.selection_order)
            .nested_queries(&q.nested)
            .builder(builder)
            .maybe_original_name(object_name)
            .call(),
        ReadQuery::AggregateRecordsQuery(q) => {
            get_result_node_for_aggregation(&q.selectors, &q.selection_order, builder, object_name)
        }
    }
}

fn map_write_query(query: &WriteQuery, builder: &mut ResultNodeBuilder) -> Option<ResultNode> {
    match query {
        WriteQuery::CreateRecord(q) => get_result_node()
            .field_selection(&q.selected_fields)
            .selection_order(&q.selection_order)
            .builder(builder)
            .call(),
        WriteQuery::CreateManyRecords(q) => get_result_node_for_create_many(q.selected_fields.as_ref(), builder),
        WriteQuery::UpdateRecord(u) => {
            match u {
                UpdateRecord::WithSelection(w) => get_result_node()
                    .field_selection(&w.selected_fields)
                    .selection_order(&w.selection_order)
                    .builder(builder)
                    .call(),
                UpdateRecord::WithoutSelection(_) => None, // No result data
            }
        }
        WriteQuery::DeleteRecord(q) => get_result_node_for_delete(q.selected_fields.as_ref(), builder),
        WriteQuery::UpdateManyRecords(q) => get_result_node_for_update_many(q.selected_fields.as_ref(), builder),
        WriteQuery::DeleteManyRecords(_) => None, // No result data
        WriteQuery::ConnectRecords(_) => None,    // No result data
        WriteQuery::DisconnectRecords(_) => None, // No result data
        WriteQuery::ExecuteRaw(_) => None,        // No data mapping
        WriteQuery::QueryRaw(_) => None,          // No data mapping
        WriteQuery::Upsert(q) => get_result_node()
            .field_selection(&q.selected_fields)
            .selection_order(&q.selection_order)
            .builder(builder)
            .call(),
    }
}

#[builder]
fn get_result_node(
    field_selection: &FieldSelection,
    selection_order: &[String],
    #[builder(default = &[])] nested_queries: &[ReadQuery],
    /// Indicates whether the query uses `relationJoins`. When true, the data mapper uses prisma names
    /// for fields rather than db names.
    #[builder(default = false)]
    uses_relation_joins: bool,
    #[builder(default = false)] is_nested: bool,
    /// Indicates whether we should skip `null` values when deserializing arrays of objects.
    /// This is a workaround for a bug in the Prisma relation mode.
    /// See https://github.com/prisma/prisma/issues/16390.
    #[builder(default = false)]
    skip_nulls: bool,
    builder: &mut ResultNodeBuilder<'_>,
    original_name: Option<Cow<'static, str>>,
) -> Option<ResultNode> {
    let field_map = field_selection
        .selections()
        .map(|fs| (fs.prisma_name_grouping_virtuals(), fs))
        .collect::<HashMap<_, _>>();
    let grouped_virtuals = field_selection
        .virtuals()
        .into_group_map_by(|vs| vs.serialized_group_name());
    let nested_map = nested_queries
        .iter()
        .map(|q| (q.get_alias_or_name(), q))
        .collect::<HashMap<_, _>>();

    let mut node = ResultNodeBuilder::new_object(original_name);
    node.set_skip_nulls(skip_nulls);

    for prisma_name in selection_order {
        match field_map.get(prisma_name.as_str()) {
            Some(SelectedField::Scalar(field)) => {
                node.add_field(
                    field.name().to_owned(),
                    get_scalar_field_result_node(field, uses_relation_joins, is_nested, builder),
                );
            }
            Some(SelectedField::Composite(_)) => todo!("MongoDB specific"),
            Some(SelectedField::Relation(f)) => {
                let nested_selection = FieldSelection::new(f.selections.to_vec());
                let original_name = if uses_relation_joins {
                    f.field.name().to_owned().into()
                } else {
                    binding::nested_relation_field(&f.field)
                };
                let nested_node = get_result_node()
                    .field_selection(&nested_selection)
                    .selection_order(&f.result_fields)
                    .builder(builder)
                    .original_name(original_name)
                    .uses_relation_joins(uses_relation_joins)
                    .is_nested(true)
                    .skip_nulls(f.field.relation().is_many_to_many())
                    .call();
                if let Some(nested_node) = nested_node {
                    node.add_field(f.field.name().to_owned(), nested_node);
                }
            }
            Some(SelectedField::Virtual(f)) => {
                for vs in grouped_virtuals
                    .get(f.serialized_group_name())
                    .map(Vec::as_slice)
                    .unwrap_or_default()
                {
                    let (group_name, field_name) = vs.serialized_name();
                    let db_name = if uses_relation_joins {
                        vs.serialized_field_name().to_owned()
                    } else {
                        vs.db_alias()
                    };

                    node.entry_or_insert(group_name, uses_relation_joins.then_some(group_name))
                        .add_field(field_name.to_owned(), builder.new_value(db_name, vs.r#type().into()));
                }
            }
            None => {
                if let Some(q) = nested_map.get(prisma_name.as_str()) {
                    let nested_node = map_read_query(
                        q,
                        builder,
                        Some(if uses_relation_joins {
                            prisma_name.to_owned().into()
                        } else {
                            binding::nested_relation_field_by_name(prisma_name)
                        }),
                    );
                    if let Some(nested_node) = nested_node {
                        node.add_field(q.get_alias_or_name().to_owned(), nested_node);
                    }
                }
            }
        }
    }

    Some(node.build())
}

fn get_scalar_field_result_node(
    field: &ScalarField,
    uses_relation_joins: bool,
    is_nested: bool,
    builder: &mut ResultNodeBuilder<'_>,
) -> ResultNode {
    let from_name = if uses_relation_joins {
        field.name().to_owned()
    } else {
        field.db_name().to_owned()
    };
    let type_info = field.type_info();

    // Nested relation join fields can have different types based on the output of database
    // specific aggregation functions.
    if uses_relation_joins && is_nested {
        // JSON fields are return as deserialized objects.
        if type_info.typ.id == TypeIdentifier::Json {
            return ResultNode::Field {
                db_name: from_name.into(),
                field_type: FieldType::new(type_info.arity, FieldScalarType::Object),
            };
        }

        // MySQL returns bytes as base64 encoded strings.
        if type_info.typ.id == TypeIdentifier::Bytes && field.dm.schema.connector.flavour() == Flavour::Mysql {
            let typ = FieldScalarType::Bytes {
                encoding: ByteArrayEncoding::Base64,
            };
            return ResultNode::Field {
                db_name: from_name.into(),
                field_type: FieldType::new(type_info.arity, typ),
            };
        }

        // PostgreSQL based databases return bytes as hex encoded strings.
        if type_info.typ.id == TypeIdentifier::Bytes
            && [Flavour::Postgres, Flavour::Cockroach].contains(&field.dm.schema.connector.flavour())
        {
            let typ = FieldScalarType::Bytes {
                encoding: ByteArrayEncoding::Hex,
            };
            return ResultNode::Field {
                db_name: from_name.into(),
                field_type: FieldType::new(type_info.arity, typ),
            };
        }
    }

    builder.new_value(from_name, type_info)
}

fn get_result_node_for_aggregation(
    selectors: &[AggregationSelection],
    selection_order: &[(String, Option<Vec<String>>)],
    builder: &mut ResultNodeBuilder,
    object_name: Option<Cow<'static, str>>,
) -> Option<ResultNode> {
    let mut ordered_set = IndexSet::new();

    for (key, nested) in selection_order {
        if let Some(nested) = nested {
            for nested_key in nested {
                ordered_set.insert((Some(key.as_str()), nested_key.as_str()));
            }
        } else {
            ordered_set.insert((None, key.as_str()));
        }
    }

    let mut node = ResultNodeBuilder::new_object(object_name);

    for (underscore_name, name, db_name, typ) in selectors
        .iter()
        .flat_map(|sel| {
            sel.identifiers().map(move |ident| {
                let (name, db_name) =
                    if matches!(&sel, AggregationSelection::Count { all: Some(_), .. }) && ident.name == "all" {
                        ("_all", "_all")
                    } else {
                        (ident.name, ident.db_name)
                    };
                let type_info = FieldTypeInformation::new(ident.typ, ident.arity, None);
                (aggregate_underscore_name(sel), name, db_name, type_info)
            })
        })
        .sorted_by_key(|(underscore_name, name, _, _)| ordered_set.get_index_of(&(*underscore_name, *name)))
    {
        let value = builder.new_value(db_name.to_owned(), typ);
        if let Some(undescore_name) = underscore_name {
            node.entry_or_insert_nested(undescore_name)
                .add_field(name.to_owned(), value);
        } else {
            node.add_field(name.to_owned(), value);
        }
    }

    Some(node.build())
}

fn aggregate_underscore_name(sel: &AggregationSelection) -> Option<&'static str> {
    match sel {
        AggregationSelection::Field(_) => None,
        AggregationSelection::Count { .. } => Some(aggregations::UNDERSCORE_COUNT),
        AggregationSelection::Average(_) => Some(aggregations::UNDERSCORE_AVG),
        AggregationSelection::Sum(_) => Some(aggregations::UNDERSCORE_SUM),
        AggregationSelection::Min(_) => Some(aggregations::UNDERSCORE_MIN),
        AggregationSelection::Max(_) => Some(aggregations::UNDERSCORE_MAX),
    }
}

fn get_result_node_for_create_many(
    selected_fields: Option<&CreateManyRecordsFields>,
    builder: &mut ResultNodeBuilder,
) -> Option<ResultNode> {
    get_result_node()
        .field_selection(&selected_fields?.fields)
        .selection_order(&selected_fields?.order)
        .nested_queries(&selected_fields?.nested)
        .builder(builder)
        .call()
}

fn get_result_node_for_delete(
    selected_fields: Option<&DeleteRecordFields>,
    builder: &mut ResultNodeBuilder,
) -> Option<ResultNode> {
    get_result_node()
        .field_selection(&selected_fields?.fields)
        .selection_order(&selected_fields?.order)
        .builder(builder)
        .call()
}

fn get_result_node_for_update_many(
    selected_fields: Option<&UpdateManyRecordsFields>,
    builder: &mut ResultNodeBuilder,
) -> Option<ResultNode> {
    get_result_node()
        .field_selection(&selected_fields?.fields)
        .selection_order(&selected_fields?.order)
        .nested_queries(&selected_fields?.nested)
        .builder(builder)
        .call()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldType {
    arity: Arity,
    #[serde(flatten)]
    r#type: FieldScalarType,
}

impl FieldType {
    pub fn new(arity: impl Into<Arity>, r#type: impl Into<FieldScalarType>) -> Self {
        Self {
            arity: arity.into(),
            r#type: r#type.into(),
        }
    }
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.arity {
            Arity::Required => write!(f, "{}", self.r#type),
            Arity::List => write!(f, "{}[]", self.r#type),
            Arity::Optional => write!(f, "{}?", self.r#type),
        }
    }
}

impl From<&FieldTypeInformation> for FieldType {
    fn from(info: &FieldTypeInformation) -> Self {
        Self {
            arity: info.arity.into(),
            r#type: FieldScalarType::from(&info.typ),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum FieldScalarType {
    String,
    Int,
    #[serde(rename = "bigint")]
    BigInt,
    Float,
    Decimal,
    Boolean,
    Enum {
        name: String,
    },
    Json,
    Object,
    #[serde(rename = "datetime")]
    DateTime,
    Bytes {
        encoding: ByteArrayEncoding,
    },
    Unsupported,
}

impl fmt::Display for FieldScalarType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String => write!(f, "String"),
            Self::Int => write!(f, "Int"),
            Self::BigInt => write!(f, "BigInt"),
            Self::Float => write!(f, "Float"),
            Self::Decimal => write!(f, "Decimal"),
            Self::Boolean => write!(f, "Boolean"),
            Self::Enum { name } => write!(f, "Enum<{name}>"),
            Self::Json => write!(f, "Json"),
            Self::Object => write!(f, "Object"),
            Self::DateTime => write!(f, "DateTime"),
            Self::Bytes { .. } => write!(f, "Bytes"),
            Self::Unsupported => write!(f, "Unsupported"),
        }
    }
}

impl From<&Type> for FieldScalarType {
    fn from(typ: &Type) -> Self {
        match typ.id {
            TypeIdentifier::String => Self::String,
            TypeIdentifier::Int => Self::Int,
            TypeIdentifier::BigInt => Self::BigInt,
            TypeIdentifier::Float => Self::Float,
            TypeIdentifier::Decimal => Self::Decimal,
            TypeIdentifier::Boolean => Self::Boolean,
            TypeIdentifier::Enum(id) => Self::Enum {
                name: typ.dm.clone().zip(id).name().to_owned(),
            },
            TypeIdentifier::UUID => Self::String,
            TypeIdentifier::Json => Self::Json,
            TypeIdentifier::DateTime => Self::DateTime,
            TypeIdentifier::Bytes => Self::Bytes {
                encoding: ByteArrayEncoding::default(),
            },
            TypeIdentifier::Unsupported => Self::Unsupported,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ByteArrayEncoding {
    #[default]
    Array,
    Base64,
    Hex,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Arity {
    Required,
    Optional,
    List,
}

impl From<FieldArity> for Arity {
    fn from(arity: FieldArity) -> Self {
        match arity {
            FieldArity::Required => Self::Required,
            FieldArity::Optional => Self::Optional,
            FieldArity::List => Self::List,
        }
    }
}
