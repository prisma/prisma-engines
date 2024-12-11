use log::warn;
use psl::{
    diagnostics::{FileId, Span},
    parser_database::ParserDatabase,
    schema_ast::ast::{
        AttributePosition, CompositeTypePosition, EnumPosition, Field, FieldId, FieldPosition, FieldType, ModelId,
        ModelPosition, SchemaPosition, SourcePosition, Top, WithAttributes, WithIdentifier, WithName, WithSpan,
    },
};

use crate::LSPContext;

fn empty_spans() -> Vec<Span> {
    vec![]
}

pub fn reference_locations_for_target<T>(ctx: &LSPContext<'_, T>, target: SchemaPosition) -> Vec<Span> {
    let spans: Vec<Span> = match target {
        // Blocks
        SchemaPosition::Model(_, ModelPosition::Name(name, span))
        | SchemaPosition::Enum(_, EnumPosition::Name(name, span))
        | SchemaPosition::CompositeType(_, CompositeTypePosition::Name(name, span)) => {
            std::iter::once(span).chain(as_field_type(ctx.db, name)).collect()
        }

        SchemaPosition::DataSource(_, SourcePosition::Name(name, span)) => {
            std::iter::once(span).chain(for_native_type(ctx.db, name)).collect()
        }

        // Fields
        SchemaPosition::Model(_, ModelPosition::Field(_, FieldPosition::Type(r#type, _)))
        | SchemaPosition::CompositeType(_, CompositeTypePosition::Field(_, FieldPosition::Type(r#type, _))) => {
            as_top_name(ctx.db, r#type)
                .into_iter()
                .chain(as_field_type(ctx.db, r#type))
                .collect()
        }

        SchemaPosition::Model(model_id, ModelPosition::Field(_, FieldPosition::Name(name, span))) => {
            std::iter::once(span)
                .chain(in_block_attribute(ctx.db, name, model_id, ctx.initiating_file_id))
                .chain(in_relation_attribute(ctx.db, name, model_id, ctx.initiating_file_id))
                .collect()
        }

        // Attributes
        SchemaPosition::Model(
            model_id,
            ModelPosition::Field(
                field_id,
                FieldPosition::Attribute(_, _, AttributePosition::ArgumentValue(arg_name, arg_value, _span)),
            ),
        ) => match arg_name {
            Some("fields") => as_field_name(ctx.db, arg_value.as_str(), model_id, ctx.initiating_file_id)
                .into_iter()
                .chain(in_block_attribute(
                    ctx.db,
                    arg_value.as_str(),
                    model_id,
                    ctx.initiating_file_id,
                ))
                .collect(),
            Some("references") => {
                let field = &ctx.db.ast(ctx.initiating_file_id)[model_id][field_id];
                let referenced_model = field.field_type.name();

                let Some(ref_model_id) = ctx.db.find_model(referenced_model) else {
                    warn!("Could not find model with name: {}", referenced_model);
                    return empty_spans();
                };

                as_field_name(ctx.db, arg_value.as_str(), ref_model_id.id.1, ref_model_id.id.0)
                    .into_iter()
                    .chain(in_block_attribute(
                        ctx.db,
                        arg_value.as_str(),
                        ref_model_id.id.1,
                        ref_model_id.id.0,
                    ))
                    .collect()
            }
            _ => empty_spans(),
        },

        // ? This might make more sense to add as a definition rather than a reference
        SchemaPosition::Model(_, ModelPosition::Field(_, FieldPosition::Attribute(name, _, _)))
        | SchemaPosition::CompositeType(_, CompositeTypePosition::Field(_, FieldPosition::Attribute(name, _, _))) => {
            match ctx.datasource().map(|ds| &ds.name) {
                Some(ds_name) if name.contains(ds_name) => as_ds_name(ctx.db, ds_name)
                    .into_iter()
                    .chain(for_native_type(ctx.db, ds_name))
                    .collect(),
                _ => empty_spans(),
            }
        }

        SchemaPosition::Model(
            model_id,
            ModelPosition::ModelAttribute(_attr_name, _, AttributePosition::ArgumentValue(_, arg_val, _span)),
        ) => as_field_name(ctx.db, arg_val.as_str(), model_id, ctx.initiating_file_id)
            .into_iter()
            .chain(in_relation_attribute(
                ctx.db,
                arg_val.as_str(),
                model_id,
                ctx.initiating_file_id,
            ))
            .chain(in_block_attribute(
                ctx.db,
                arg_val.as_str(),
                model_id,
                ctx.initiating_file_id,
            ))
            .collect(),

        _ => empty_spans(),
    };

    spans
}

fn extract_ds_from_native_type(attr_name: &str) -> Option<&str> {
    attr_name.split('.').next()
}

pub fn in_relation_attribute<'ast>(
    db: &'ast ParserDatabase,
    name: &'ast str,
    model_id: ModelId,
    file_id: FileId,
) -> impl Iterator<Item = Span> + 'ast {
    let model = db.walk((file_id, model_id));

    model.relation_fields().flat_map(move |rf| {
        rf.relation_attribute()
            .and_then(|attr| {
                attr.arguments
                    .arguments
                    .iter()
                    .find(|arg| arg.name().map_or(false, |name| name == "fields") && arg.value.is_array())
                    .and_then(|arg| {
                        arg.value.as_array().and_then(|arr| {
                            arr.0
                                .iter()
                                .find(|expr| expr.as_constant_value().map_or(false, |cv| cv.0 == name))
                        })
                    })
            })
            .map(|expr| expr.span())
    })
}

pub fn in_block_attribute<'ast>(
    db: &'ast ParserDatabase,
    name: &'ast str,
    model_id: ModelId,
    file_id: FileId,
) -> impl Iterator<Item = Span> + 'ast {
    let model = db.walk((file_id, model_id));

    model.ast_model().attributes().iter().filter_map(move |attr| {
        attr.arguments
            .arguments
            .iter()
            .find_map(|arg| {
                arg.value.as_array().and_then(|arr| {
                    arr.0
                        .iter()
                        .find(|expr| expr.as_constant_value().map_or(false, |cv| cv.0 == name))
                })
            })
            .map(|arg| arg.span())
    })
}

pub fn as_field_name(db: &ParserDatabase, name: &str, model_id: ModelId, file_id: FileId) -> Option<Span> {
    let model = db.walk((file_id, model_id));

    model
        .scalar_fields()
        .find(|field| field.name() == name)
        .map(|field| field.ast_field().identifier().span())
}

pub fn for_native_type<'ast>(db: &'ast ParserDatabase, name: &'ast str) -> impl Iterator<Item = Span> + 'ast {
    fn find_native_type_locations<'ast>(
        name: &'ast str,
        fields: impl Iterator<Item = (FieldId, &'ast Field)> + 'ast,
    ) -> Box<dyn Iterator<Item = Span> + 'ast> {
        Box::new(fields.filter_map(move |field| {
            field
                .1
                .attributes
                .iter()
                .find(|attr| extract_ds_from_native_type(attr.name()) == Some(name))
                .map(|attr| attr.identifier().span())
        }))
    }

    db.walk_tops().flat_map(move |top| match top.ast_top() {
        Top::CompositeType(composite_type) => find_native_type_locations(name, composite_type.iter_fields()),
        Top::Model(model) => find_native_type_locations(name, model.iter_fields()),

        Top::Enum(_) | Top::Source(_) | Top::Generator(_) => Box::new(std::iter::empty()),
    })
}

pub fn as_field_type<'ast>(db: &'ast ParserDatabase, name: &'ast str) -> impl Iterator<Item = Span> + 'ast {
    fn get_relevent_identifiers<'a>(fields: impl Iterator<Item = (FieldId, &'a Field)>, name: &str) -> Vec<Span> {
        fields
            .filter_map(|(_id, field)| match &field.field_type {
                FieldType::Supported(id) if id.name == name => Some(id.span()),
                _ => None,
            })
            .collect()
    }

    db.walk_tops().flat_map(|top| match top.ast_top() {
        Top::Model(model) => get_relevent_identifiers(model.iter_fields(), name),
        Top::CompositeType(composite_type) => get_relevent_identifiers(composite_type.iter_fields(), name),
        // * Cannot contain field types
        Top::Enum(_) | Top::Source(_) | Top::Generator(_) => empty_spans(),
    })
}

pub fn as_top_name<'ast>(db: &'ast ParserDatabase, name: &'ast str) -> Option<Span> {
    db.find_top(name).map(|top| top.ast_top().identifier().span())
}

pub fn as_ds_name<'ast>(db: &'ast ParserDatabase, name: &'ast str) -> Option<Span> {
    db.find_source(name)
        .map(|source| db.ast(source.0)[source.1].identifier().span())
}
