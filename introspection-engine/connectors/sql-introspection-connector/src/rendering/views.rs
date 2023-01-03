use std::{borrow::Cow, collections::HashMap};

use crate::datamodel_calculator::{InputContext, OutputContext};
use datamodel_renderer::{
    datamodel as renderer,
    value::{Array, Function, FunctionParam, Text, Value},
};
use either::Either;
use psl::{
    parser_database::{
        walkers::{IndexWalker, PrimaryKeyWalker, RelationFieldWalker, RelationName, ScalarFieldWalker},
        IndexType, SortOrder,
    },
    schema_ast::ast::{self, FieldId, WithDocumentation},
};

pub(crate) fn render<'a>(input: InputContext<'a>, output: &mut OutputContext<'a>) {
    for view in input
        .previous_schema
        .db
        .walk_models()
        .filter(|m| m.ast_model().is_view())
    {
        let mut rendered = renderer::View::new(view.name());

        if let Some(docs) = view.ast_model().documentation() {
            rendered.documentation(docs);
        }

        if let Some(map) = view.mapped_name() {
            rendered.map(map);
        }

        if let Some(ns) = view.schema_name() {
            rendered.schema(ns);
        }

        if let Some(pk) = view.primary_key().filter(|pk| pk.fields().len() > 1) {
            let fields = pk.scalar_field_attributes().map(|sf| {
                let mut definition = renderer::IndexFieldInput::new(sf.as_path_string());

                if let Some(length) = sf.length() {
                    definition.length(length)
                };

                if let Some(sort) = sf.sort_order() {
                    match sort {
                        SortOrder::Asc => definition.sort_order("Asc"),
                        SortOrder::Desc => definition.sort_order("Desc"),
                    }
                };

                definition
            });

            let mut id = renderer::IdDefinition::new(fields);

            if let Some(name) = pk.name() {
                id.name(name);
            }

            if let Some(map) = pk.mapped_name() {
                id.map(map);
            }

            if let Some(clustered) = pk.clustered() {
                id.clustered(clustered);
            }

            rendered.id(id);
        }

        if view.is_ignored() {
            rendered.ignore();
        }

        let id_field = view.primary_key().filter(|pk| pk.fields().len() == 1);

        let uniques: HashMap<_, _> = view
            .indexes()
            .filter(|i| i.is_unique())
            .filter(|i| i.fields().len() == 1)
            .map(|i| {
                let f = i.fields().next().unwrap();
                (f.field_id(), i)
            })
            .collect();

        for sf in view.scalar_fields() {
            rendered.push_field(render_scalar_field(sf, id_field, &uniques));
        }

        for rf in view.relation_fields() {
            rendered.push_field(render_relation_field(rf));
        }

        for index in view.indexes().filter(|i| !(i.is_unique() && i.fields().len() == 1)) {
            rendered.push_index(render_index(index));
        }

        output.rendered_schema.push_view(rendered);
    }
}

fn render_scalar_field<'a>(
    sf: ScalarFieldWalker<'a>,
    id_field: Option<PrimaryKeyWalker<'a>>,
    uniques: &HashMap<FieldId, IndexWalker<'a>>,
) -> renderer::Field<'a> {
    let mut rendered_field = renderer::Field::new(sf.name(), sf.type_str());

    match sf.ast_field().arity {
        ast::FieldArity::Required => (),
        ast::FieldArity::Optional => rendered_field.optional(),
        ast::FieldArity::List => rendered_field.array(),
    }

    if sf.scalar_field_type().is_unsupported() {
        rendered_field.unsupported();
    }

    if let Some(map) = sf.mapped_name() {
        rendered_field.map(map);
    }

    if let Some((prefix, name, args, _)) = sf.raw_native_type() {
        rendered_field.native_type(prefix, name, args.to_vec());
    }

    if let Some(docs) = sf.ast_field().documentation() {
        rendered_field.documentation(docs);
    }

    if sf.is_ignored() {
        rendered_field.ignore();
    }

    if sf.is_updated_at() {
        rendered_field.updated_at();
    }

    if let Some(id) = id_field.filter(|pk| pk.fields().any(|f| f.field_id() == sf.field_id())) {
        let field = id.scalar_field_attributes().next().unwrap();
        let mut definition = renderer::IdFieldDefinition::new();

        if let Some(clustered) = id.clustered() {
            definition.clustered(clustered);
        }

        if let Some(sort_order) = field.sort_order() {
            match sort_order {
                SortOrder::Asc => definition.sort_order("Asc"),
                SortOrder::Desc => definition.sort_order("Desc"),
            }
        }

        if let Some(length) = field.length() {
            definition.length(length);
        }

        if let Some(map) = id.mapped_name() {
            definition.map(map);
        }

        rendered_field.id(definition);
    }

    if let Some(unique) = uniques.get(&sf.field_id()) {
        let mut opts = renderer::UniqueFieldAttribute::default();
        let field = unique.scalar_field_attributes().next().unwrap();

        if let Some(map) = unique.mapped_name() {
            opts.map(map);
        }

        if let Some(clustered) = unique.clustered() {
            opts.clustered(clustered);
        }

        if let Some(sort_order) = field.sort_order() {
            match sort_order {
                SortOrder::Asc => opts.sort_order("Asc"),
                SortOrder::Desc => opts.sort_order("Desc"),
            }
        }

        if let Some(length) = field.length() {
            opts.length(length);
        }

        rendered_field.unique(opts);
    }

    if let Some(df) = sf.default_value() {
        let mut opts = expr_to_default_value(df.value());

        if let Some(map) = df.mapped_name() {
            opts.map(map);
        }

        rendered_field.default(opts);
    }

    rendered_field
}

fn render_relation_field<'a>(rf: RelationFieldWalker<'a>) -> renderer::Field<'a> {
    let mut rendered = renderer::Field::new(rf.name(), rf.related_model().name());

    if rf.ast_field().arity.is_optional() {
        rendered.optional();
    } else if rf.ast_field().arity.is_list() {
        rendered.array();
    }

    if rf.is_ignored() {
        rendered.ignore();
    }

    let renders_attribute =
        rf.relation_name().is_explicit() || rf.fields().is_some() || rf.referenced_fields().is_some();

    if renders_attribute {
        let mut relation = renderer::Relation::new();

        if let RelationName::Explicit(name) = rf.relation_name() {
            relation.name(name);
        }

        if let Some(fields) = rf.fields() {
            let fields = fields.map(|f| f.name()).map(Cow::Borrowed);
            relation.fields(fields);
        }

        if let Some(references) = rf.referenced_fields() {
            let references = references.map(|f| f.name()).map(Cow::Borrowed);
            relation.fields(references);
        }

        if let Some(action) = rf.explicit_on_delete() {
            relation.on_delete(action.as_str());
        }

        if let Some(action) = rf.explicit_on_update() {
            relation.on_update(action.as_str());
        }

        if let Some(map) = rf.mapped_name() {
            relation.map(map);
        }

        rendered.relation(relation);
    }

    rendered
}

fn render_index<'a>(index: IndexWalker<'a>) -> renderer::IndexDefinition<'a> {
    let fields = index.scalar_field_attributes().map(|field| {
        let mut definition = renderer::IndexFieldInput::new(field.as_index_field().name());

        if let Some(sort_order) = field.sort_order() {
            match sort_order {
                SortOrder::Asc => definition.sort_order("Asc"),
                SortOrder::Desc => definition.sort_order("Desc"),
            }
        }

        if let Some(length) = field.length() {
            definition.length(length);
        }

        if let Some(ops) = field.operator_class() {
            match ops.get() {
                Either::Left(managed) => definition.ops(renderer::IndexOps::managed(managed.as_str())),
                Either::Right(raw) => definition.ops(renderer::IndexOps::raw(raw)),
            };
        }

        definition
    });

    let mut definition = match index.index_type() {
        IndexType::Normal => renderer::IndexDefinition::index(fields),
        IndexType::Unique => renderer::IndexDefinition::unique(fields),
        IndexType::Fulltext => renderer::IndexDefinition::fulltext(fields),
    };

    if let Some(name) = index.name() {
        definition.name(name);
    }

    if let Some(map) = index.mapped_name() {
        definition.map(map);
    }

    if let Some(clustered) = index.clustered() {
        definition.clustered(clustered);
    }

    if let Some(algo) = index.algorithm() {
        definition.index_type(algo.as_str())
    }

    definition
}

fn expr_to_default_value<'a>(expr: &'a ast::Expression) -> renderer::DefaultValue<'a> {
    match expr {
        ast::Expression::NumericValue(s, _) => renderer::DefaultValue::constant(s),
        ast::Expression::StringValue(s, _) => renderer::DefaultValue::text(s),
        ast::Expression::ConstantValue(s, _) => renderer::DefaultValue::constant(s),
        ast::Expression::Function(name, args, _) => {
            let mut fun = Function::new(name);

            for arg in args.arguments.iter() {
                let fun_param = match arg.name {
                    Some(ref name) => FunctionParam::from((name.name.as_str(), expr_to_value(&arg.value))),
                    None => FunctionParam::from(expr_to_value(&arg.value)),
                };

                fun.push_param(fun_param);
            }

            renderer::DefaultValue::function(fun)
        }
        ast::Expression::Array(exprs, _) => {
            let values = exprs.iter().map(expr_to_value).collect();
            renderer::DefaultValue::array(values)
        }
    }
}

fn expr_to_value<'a>(expr: &'a ast::Expression) -> Value<'a> {
    match expr {
        ast::Expression::NumericValue(s, _) => Value::Constant(s.into()),
        ast::Expression::StringValue(s, _) => Value::Text(Text::new(s)),
        ast::Expression::ConstantValue(s, _) => Value::Constant(s.into()),
        ast::Expression::Function(_, _, _) => unreachable!(),
        ast::Expression::Array(exprs, _) => {
            let vals: Vec<_> = exprs.iter().map(expr_to_value).collect();
            Value::Array(Array::from(vals))
        }
    }
}
