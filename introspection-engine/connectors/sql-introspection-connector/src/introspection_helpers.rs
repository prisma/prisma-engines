use crate::{calculate_datamodel::CalculateDatamodelContext as Context, defaults::render_default, SqlFamilyTrait};
use datamodel_renderer::datamodel as renderer;
use psl::{
    datamodel_connector::constraint_names::ConstraintNames, parser_database::walkers,
    schema_ast::ast::WithDocumentation, PreviewFeature,
};
use sql::{
    walkers::{ColumnWalker, TableWalker},
    IndexWalker,
};
use sql_schema_describer::{
    self as sql, mssql::MssqlSchemaExt, postgres::PostgresSchemaExt, ColumnArity, ColumnTypeFamily, IndexType,
};
use std::{borrow::Cow, cmp};

/// This function implements the reverse behaviour of the `Ord` implementation for `Option`: it
/// puts `None` values last, and otherwise orders `Some`s by their contents, like the `Ord` impl.
pub(crate) fn compare_options_none_last<T: cmp::Ord>(a: Option<T>, b: Option<T>) -> cmp::Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => cmp::Ordering::Less,
        (None, Some(_)) => cmp::Ordering::Greater,
        (None, None) => cmp::Ordering::Equal,
    }
}

pub(crate) fn is_old_migration_table(table: TableWalker<'_>) -> bool {
    table.name() == "_Migration"
        && table.columns().any(|c| c.name() == "revision")
        && table.columns().any(|c| c.name() == "name")
        && table.columns().any(|c| c.name() == "datamodel")
        && table.columns().any(|c| c.name() == "status")
        && table.columns().any(|c| c.name() == "applied")
        && table.columns().any(|c| c.name() == "rolled_back")
        && table.columns().any(|c| c.name() == "datamodel_steps")
        && table.columns().any(|c| c.name() == "database_migration")
        && table.columns().any(|c| c.name() == "errors")
        && table.columns().any(|c| c.name() == "started_at")
        && table.columns().any(|c| c.name() == "finished_at")
}

pub(crate) fn is_new_migration_table(table: TableWalker<'_>) -> bool {
    table.name() == "_prisma_migrations"
        && table.columns().any(|c| c.name() == "id")
        && table.columns().any(|c| c.name() == "checksum")
        && table.columns().any(|c| c.name() == "finished_at")
        && table.columns().any(|c| c.name() == "migration_name")
        && table.columns().any(|c| c.name() == "logs")
        && table.columns().any(|c| c.name() == "rolled_back_at")
        && table.columns().any(|c| c.name() == "started_at")
        && table.columns().any(|c| c.name() == "applied_steps_count")
}

pub(crate) fn is_relay_table(table: TableWalker<'_>) -> bool {
    table.name() == "_RelayId"
        && table.column("id").is_some()
        && table
            .columns()
            .any(|col| col.name().eq_ignore_ascii_case("stablemodelidentifier"))
}

pub(crate) fn has_created_at_and_updated_at(table: TableWalker<'_>) -> bool {
    let has_created_at = table.columns().any(|col| {
        col.name().eq_ignore_ascii_case("createdat") && col.column_type().family == ColumnTypeFamily::DateTime
    });

    let has_updated_at = table.columns().any(|col| {
        col.name().eq_ignore_ascii_case("updatedat") && col.column_type().family == ColumnTypeFamily::DateTime
    });

    has_created_at && has_updated_at
}

pub(crate) fn is_prisma_join_table(t: TableWalker<'_>) -> bool {
    is_prisma_1_point_0_join_table(t) || is_prisma_1_point_1_or_2_join_table(t)
}

pub(crate) fn is_prisma_1_or_11_list_table(table: TableWalker<'_>) -> bool {
    table.columns().len() == 3
        && table.columns().any(|col| col.name().eq_ignore_ascii_case("nodeid"))
        && table.column("position").is_some()
        && table.column("value").is_some()
}

pub(crate) fn is_prisma_1_point_1_or_2_join_table(table: TableWalker<'_>) -> bool {
    table.columns().len() == 2 && table.indexes().len() >= 2 && common_prisma_m_to_n_relation_conditions(table)
}

pub(crate) fn is_prisma_1_point_0_join_table(table: TableWalker<'_>) -> bool {
    table.columns().len() == 3
        && table.indexes().len() >= 2
        && table.columns().any(|c| c.name() == "id")
        && common_prisma_m_to_n_relation_conditions(table)
}

fn common_prisma_m_to_n_relation_conditions(table: TableWalker<'_>) -> bool {
    fn is_a(column: &str) -> bool {
        column.eq_ignore_ascii_case("a")
    }

    fn is_b(column: &str) -> bool {
        column.eq_ignore_ascii_case("b")
    }

    let mut fks = table.foreign_keys();
    let first_fk = fks.next();
    let second_fk = fks.next();
    let a_b_match = || {
        let first_fk = first_fk.unwrap();
        let second_fk = second_fk.unwrap();
        let first_fk_col = first_fk.constrained_columns().next().unwrap().name();
        let second_fk_col = second_fk.constrained_columns().next().unwrap().name();
        (first_fk.referenced_table().name() <= second_fk.referenced_table().name()
            && is_a(first_fk_col)
            && is_b(second_fk_col))
            || (second_fk.referenced_table().name() <= first_fk.referenced_table().name()
                && is_b(first_fk_col)
                && is_a(second_fk_col))
    };
    table.name().starts_with('_')
        //UNIQUE INDEX [A,B]
        && table.indexes().any(|i| {
            i.columns().len() == 2
                && is_a(i.columns().next().unwrap().as_column().name())
                && is_b(i.columns().nth(1).unwrap().as_column().name())
                && i.is_unique()
        })
    //INDEX [B]
    && table
        .indexes()
        .any(|i| i.columns().len() == 1 && is_b(i.columns().next().unwrap().as_column().name()) && i.index_type() == IndexType::Normal)
        // 2 FKs
        && table.foreign_keys().len() == 2
        // Lexicographically lower model referenced by A
        && a_b_match()
}

//calculators

pub(crate) fn render_index<'a>(
    index: sql::IndexWalker<'a>,
    existing_index: Option<walkers::IndexWalker<'a>>,
    ctx: &Context<'a>,
) -> Option<renderer::IndexDefinition<'a>> {
    let fields = index.columns().map(|col| {
        let name = ctx.column_prisma_name(col.as_column().id).prisma_name();
        let mut definition = renderer::IndexFieldInput::new(name);

        if col
            .sort_order()
            .filter(|so| matches!(so, sql::SQLSortOrder::Desc))
            .is_some()
        {
            definition.sort_order("Desc");
        }

        if let Some(length) = col.length() {
            definition.length(length);
        }

        if let Some(ops) = render_opclass(col.id, ctx) {
            definition.ops(ops);
        }

        definition
    });

    let mut definition = match index.index_type() {
        // we handle these in the field level
        sql::IndexType::Unique if fields.len() == 1 => {
            return None;
        }
        sql::IndexType::Unique => renderer::IndexDefinition::unique(fields),
        sql::IndexType::Fulltext if ctx.config.preview_features().contains(PreviewFeature::FullTextIndex) => {
            renderer::IndexDefinition::fulltext(fields)
        }
        sql::IndexType::Normal | sql::IndexType::Fulltext => renderer::IndexDefinition::index(fields),
        // we handle these in the model id definition
        sql::IndexType::PrimaryKey => return None,
    };

    let default_constraint_name = match index.index_type() {
        IndexType::Unique => {
            let columns = index.column_names().collect::<Vec<_>>();
            ConstraintNames::unique_index_name(index.table().name(), &columns, ctx.active_connector())
        }
        _ => {
            let columns = index.column_names().collect::<Vec<_>>();
            ConstraintNames::non_unique_index_name(index.table().name(), &columns, ctx.active_connector())
        }
    };

    if let Some(name) = existing_index.and_then(|idx| idx.name()) {
        definition.name(name);
    }

    if index.name() != default_constraint_name {
        definition.map(index.name());
    }

    if let Some(clustered) = index_is_clustered(index.id, ctx) {
        definition.clustered(clustered);
    }

    if let Some(algo) = render_index_algorithm(index, ctx) {
        definition.index_type(algo);
    }

    Some(definition)
}

pub(crate) fn render_scalar_field<'a>(
    column: ColumnWalker<'a>,
    primary_key: Option<IndexWalker<'a>>,
    unique: Option<IndexWalker<'a>>,
    ctx: &mut Context<'a>,
) -> renderer::ModelField<'a> {
    let existing_field = ctx.existing_scalar_field(column.id);

    let (name, database_name, docs_for_commenting_out, is_commented_out) = {
        let names = ctx.column_prisma_name(column.id);
        let prisma_name = names.prisma_name();
        let mapped_name = names.mapped_name();

        if prisma_name.is_empty() {
            ctx.fields_with_empty_names.push(crate::warnings::ModelAndField {
                model: ctx.table_prisma_name(column.table().id).prisma_name().into_owned(),
                field: column.name().to_owned(),
            });

            let docs = "This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*";

            (
                mapped_name.map(Cow::from).unwrap_or(prisma_name),
                mapped_name,
                Some(docs),
                true,
            )
        } else {
            (prisma_name, mapped_name, None, false)
        }
    };

    if let Some(field) = existing_field.filter(|f| f.mapped_name().is_some()) {
        ctx.remapped_fields.push(crate::warnings::ModelAndField {
            model: field.model().name().to_owned(),
            field: field.name().to_owned(),
        });
    }

    let column_type = match column.column_type_family() {
        ColumnTypeFamily::Int => Cow::from("Int"),
        ColumnTypeFamily::BigInt => Cow::from("BigInt"),
        ColumnTypeFamily::Float => Cow::from("Float"),
        ColumnTypeFamily::Decimal => Cow::from("Decimal"),
        ColumnTypeFamily::Boolean => Cow::from("Boolean"),
        ColumnTypeFamily::String => Cow::from("String"),
        ColumnTypeFamily::DateTime => Cow::from("DateTime"),
        ColumnTypeFamily::Binary => Cow::from("Bytes"),
        ColumnTypeFamily::Json => Cow::from("Json"),
        ColumnTypeFamily::Uuid => Cow::from("String"),
        ColumnTypeFamily::Enum(id) => ctx.enum_prisma_name(*id).prisma_name(),
        ColumnTypeFamily::Unsupported(ref typ) => Cow::from(typ),
    };

    let mut field = match column.column_type().arity {
        ColumnArity::Required if column.column_type_family().is_unsupported() => {
            renderer::ModelField::new_required_unsupported(name, column_type)
        }
        ColumnArity::Nullable if column.column_type_family().is_unsupported() => {
            renderer::ModelField::new_optional_unsupported(name, column_type)
        }
        ColumnArity::List if column.column_type_family().is_unsupported() => {
            renderer::ModelField::new_array_unsupported(name, column_type)
        }
        ColumnArity::Required => renderer::ModelField::new_required(name, column_type),
        ColumnArity::Nullable => renderer::ModelField::new_optional(name, column_type),
        ColumnArity::List => renderer::ModelField::new_array(name, column_type),
    };

    let types =
        calculate_psl_scalar_type(column).and_then(|st| column.column_type().native_type.as_ref().map(|nt| (st, nt)));

    if let Some((scalar_type, native_type)) = types {
        let is_default = ctx
            .active_connector()
            .native_type_is_default_for_scalar_type(native_type, &scalar_type);

        if !is_default {
            let (r#type, params) = ctx.active_connector().native_type_to_parts(native_type);
            let prefix = &ctx.config.datasources.first().unwrap().name;

            field.native_type(prefix, r#type, params)
        }
    }

    if let Some(pk) = primary_key {
        let mut id_field = renderer::IdFieldDefinition::default();
        let col = pk.columns().next().unwrap();

        if let Some(clustered) = primary_key_is_clustered(pk.id, ctx) {
            id_field.clustered(clustered);
        }

        if col
            .sort_order()
            .filter(|o| matches!(o, sql::SQLSortOrder::Desc))
            .is_some()
        {
            id_field.sort_order("Desc");
        }

        if let Some(length) = col.length() {
            id_field.length(length);
        }

        let default_name = ConstraintNames::primary_key_name(column.table().name(), ctx.active_connector());
        if pk.name() != default_name && !pk.name().is_empty() {
            id_field.map(pk.name());
        }

        field.id(id_field);
    }

    if let Some(unique) = unique {
        let mut opts = renderer::IndexFieldOptions::default();
        let col = unique.columns().next().unwrap();

        let default_constraint_name =
            ConstraintNames::unique_index_name(unique.table().name(), &[col.name()], ctx.active_connector());

        if unique.name() != default_constraint_name {
            opts.map(unique.name());
        }

        if let Some(clustered) = index_is_clustered(unique.id, ctx) {
            opts.clustered(clustered);
        }

        if col
            .sort_order()
            .filter(|o| matches!(o, sql::SQLSortOrder::Desc))
            .is_some()
        {
            opts.sort_order("Desc");
        }

        if let Some(length) = col.length() {
            opts.length(length);
        }

        field.unique(opts);
    }

    if let Some(docs) = existing_field.and_then(|f| f.ast_field().documentation()) {
        field.documentation(docs);
    }

    if let Some(docs) = docs_for_commenting_out {
        field.documentation(docs);
    }

    if is_commented_out {
        field.commented_out();
    }

    if let Some(default) = render_default(column, existing_field, ctx) {
        field.default(default);
    }

    if let Some(map) = database_name {
        field.map(map);
    }

    if existing_field.map(|f| f.is_updated_at()).unwrap_or(false) {
        field.updated_at();
    }

    if existing_field.map(|f| f.is_ignored()).unwrap_or(false) {
        field.ignore();
    }

    field
}

pub(crate) fn calculate_psl_scalar_type(column: ColumnWalker<'_>) -> Option<psl::parser_database::ScalarType> {
    match column.column_type_family() {
        ColumnTypeFamily::Int => Some(psl::parser_database::ScalarType::Int),
        ColumnTypeFamily::BigInt => Some(psl::parser_database::ScalarType::BigInt),
        ColumnTypeFamily::Float => Some(psl::parser_database::ScalarType::Float),
        ColumnTypeFamily::Decimal => Some(psl::parser_database::ScalarType::Decimal),
        ColumnTypeFamily::Boolean => Some(psl::parser_database::ScalarType::Boolean),
        ColumnTypeFamily::String => Some(psl::parser_database::ScalarType::String),
        ColumnTypeFamily::DateTime => Some(psl::parser_database::ScalarType::DateTime),
        ColumnTypeFamily::Json => Some(psl::parser_database::ScalarType::Json),
        ColumnTypeFamily::Uuid => Some(psl::parser_database::ScalarType::String),
        ColumnTypeFamily::Binary => Some(psl::parser_database::ScalarType::Bytes),
        ColumnTypeFamily::Enum(_) => None,
        ColumnTypeFamily::Unsupported(_) => None,
    }
}

// misc

fn render_index_algorithm(index: sql::walkers::IndexWalker<'_>, ctx: &Context) -> Option<&'static str> {
    if !ctx.sql_family().is_postgres() {
        return None;
    }

    let data: &PostgresSchemaExt = index.schema.downcast_connector_data();

    match data.index_algorithm(index.id) {
        sql::postgres::SqlIndexAlgorithm::BTree => None,
        sql::postgres::SqlIndexAlgorithm::Hash => Some("Hash"),
        sql::postgres::SqlIndexAlgorithm::Gist => Some("Gist"),
        sql::postgres::SqlIndexAlgorithm::Gin => Some("Gin"),
        sql::postgres::SqlIndexAlgorithm::SpGist => Some("SpGist"),
        sql::postgres::SqlIndexAlgorithm::Brin => Some("Brin"),
    }
}

fn index_is_clustered(index_id: sql::IndexId, ctx: &Context) -> Option<bool> {
    if !ctx.sql_family().is_mssql() {
        return None;
    }

    let ext: &MssqlSchemaExt = ctx.schema.downcast_connector_data();
    let clustered = ext.index_is_clustered(index_id);

    if !clustered {
        return None;
    }

    Some(clustered)
}

pub(crate) fn primary_key_is_clustered(pkid: sql::IndexId, ctx: &Context) -> Option<bool> {
    if !ctx.sql_family().is_mssql() {
        return None;
    }

    let ext: &MssqlSchemaExt = ctx.schema.downcast_connector_data();

    let clustered = ext.index_is_clustered(pkid);

    if clustered {
        return None;
    }

    Some(clustered)
}

fn render_opclass<'a>(index_field_id: sql::IndexColumnId, ctx: &Context<'a>) -> Option<renderer::IndexOps<'a>> {
    if !ctx.sql_family().is_postgres() {
        return None;
    }

    let ext: &PostgresSchemaExt = ctx.schema.downcast_connector_data();

    let opclass = match ext.get_opclass(index_field_id) {
        Some(opclass) => opclass,
        None => return None,
    };

    match &opclass.kind {
        _ if opclass.is_default => None,
        sql::postgres::SQLOperatorClassKind::InetOps => Some(renderer::IndexOps::managed("InetOps")),
        sql::postgres::SQLOperatorClassKind::JsonbOps => Some(renderer::IndexOps::managed("JsonbOps")),
        sql::postgres::SQLOperatorClassKind::JsonbPathOps => Some(renderer::IndexOps::managed("JsonbPathOps")),
        sql::postgres::SQLOperatorClassKind::ArrayOps => Some(renderer::IndexOps::managed("ArrayOps")),
        sql::postgres::SQLOperatorClassKind::TextOps => Some(renderer::IndexOps::managed("TextOps")),
        sql::postgres::SQLOperatorClassKind::BitMinMaxOps => Some(renderer::IndexOps::managed("BitMinMaxOps")),
        sql::postgres::SQLOperatorClassKind::VarBitMinMaxOps => Some(renderer::IndexOps::managed("VarBitMinMaxOps")),
        sql::postgres::SQLOperatorClassKind::BpcharBloomOps => Some(renderer::IndexOps::managed("BpcharBloomOps")),
        sql::postgres::SQLOperatorClassKind::BpcharMinMaxOps => Some(renderer::IndexOps::managed("BpcharMinMaxOps")),
        sql::postgres::SQLOperatorClassKind::ByteaBloomOps => Some(renderer::IndexOps::managed("ByteaBloomOps")),
        sql::postgres::SQLOperatorClassKind::ByteaMinMaxOps => Some(renderer::IndexOps::managed("ByteaMinMaxOps")),
        sql::postgres::SQLOperatorClassKind::DateBloomOps => Some(renderer::IndexOps::managed("DateBloomOps")),
        sql::postgres::SQLOperatorClassKind::DateMinMaxOps => Some(renderer::IndexOps::managed("DateMinMaxOps")),
        sql::postgres::SQLOperatorClassKind::DateMinMaxMultiOps => {
            Some(renderer::IndexOps::managed("DateMinMaxMultiOps"))
        }
        sql::postgres::SQLOperatorClassKind::Float4BloomOps => Some(renderer::IndexOps::managed("Float4BloomOps")),
        sql::postgres::SQLOperatorClassKind::Float4MinMaxOps => Some(renderer::IndexOps::managed("Float4MinMaxOps")),
        sql::postgres::SQLOperatorClassKind::Float4MinMaxMultiOps => {
            Some(renderer::IndexOps::managed("Float4MinMaxMultiOps"))
        }
        sql::postgres::SQLOperatorClassKind::Float8BloomOps => Some(renderer::IndexOps::managed("Float8BloomOps")),
        sql::postgres::SQLOperatorClassKind::Float8MinMaxOps => Some(renderer::IndexOps::managed("Float8MinMaxOps")),
        sql::postgres::SQLOperatorClassKind::Float8MinMaxMultiOps => {
            Some(renderer::IndexOps::managed("Float8MinMaxMultiOps"))
        }
        sql::postgres::SQLOperatorClassKind::InetInclusionOps => Some(renderer::IndexOps::managed("InetInclusionOps")),
        sql::postgres::SQLOperatorClassKind::InetBloomOps => Some(renderer::IndexOps::managed("InetBloomOps")),
        sql::postgres::SQLOperatorClassKind::InetMinMaxOps => Some(renderer::IndexOps::managed("InetMinMaxOps")),
        sql::postgres::SQLOperatorClassKind::InetMinMaxMultiOps => {
            Some(renderer::IndexOps::managed("InetMinMaxMultiOps"))
        }
        sql::postgres::SQLOperatorClassKind::Int2BloomOps => Some(renderer::IndexOps::managed("Int2BloomOps")),
        sql::postgres::SQLOperatorClassKind::Int2MinMaxOps => Some(renderer::IndexOps::managed("Int2MinMaxOps")),
        sql::postgres::SQLOperatorClassKind::Int2MinMaxMultiOps => {
            Some(renderer::IndexOps::managed("Int2MinMaxMultiOps"))
        }
        sql::postgres::SQLOperatorClassKind::Int4BloomOps => Some(renderer::IndexOps::managed("Int4BloomOps")),
        sql::postgres::SQLOperatorClassKind::Int4MinMaxOps => Some(renderer::IndexOps::managed("Int4MinMaxOps")),
        sql::postgres::SQLOperatorClassKind::Int4MinMaxMultiOps => {
            Some(renderer::IndexOps::managed("Int4MinMaxMultiOps"))
        }
        sql::postgres::SQLOperatorClassKind::Int8BloomOps => Some(renderer::IndexOps::managed("Int8BloomOps")),
        sql::postgres::SQLOperatorClassKind::Int8MinMaxOps => Some(renderer::IndexOps::managed("Int8MinMaxOps")),
        sql::postgres::SQLOperatorClassKind::Int8MinMaxMultiOps => {
            Some(renderer::IndexOps::managed("Int8MinMaxMultiOps"))
        }
        sql::postgres::SQLOperatorClassKind::NumericBloomOps => Some(renderer::IndexOps::managed("NumericBloomOps")),
        sql::postgres::SQLOperatorClassKind::NumericMinMaxOps => Some(renderer::IndexOps::managed("NumericMinMaxOps")),
        sql::postgres::SQLOperatorClassKind::NumericMinMaxMultiOps => {
            Some(renderer::IndexOps::managed("NumericMinMaxMultiOps"))
        }
        sql::postgres::SQLOperatorClassKind::OidBloomOps => Some(renderer::IndexOps::managed("OidBloomOps")),
        sql::postgres::SQLOperatorClassKind::OidMinMaxOps => Some(renderer::IndexOps::managed("OidMinMaxOps")),
        sql::postgres::SQLOperatorClassKind::OidMinMaxMultiOps => {
            Some(renderer::IndexOps::managed("OidMinMaxMultiOps"))
        }
        sql::postgres::SQLOperatorClassKind::TextBloomOps => Some(renderer::IndexOps::managed("TextBloomOps")),
        sql::postgres::SQLOperatorClassKind::TextMinMaxOps => Some(renderer::IndexOps::managed("TextMinMaxOps")),
        sql::postgres::SQLOperatorClassKind::TimestampBloomOps => {
            Some(renderer::IndexOps::managed("TimestampBloomOps"))
        }
        sql::postgres::SQLOperatorClassKind::TimestampMinMaxOps => {
            Some(renderer::IndexOps::managed("TimestampMinMaxOps"))
        }
        sql::postgres::SQLOperatorClassKind::TimestampMinMaxMultiOps => {
            Some(renderer::IndexOps::managed("TimestampMinMaxMultiOps"))
        }
        sql::postgres::SQLOperatorClassKind::TimestampTzBloomOps => {
            Some(renderer::IndexOps::managed("TimestampTzBloomOps"))
        }
        sql::postgres::SQLOperatorClassKind::TimestampTzMinMaxOps => {
            Some(renderer::IndexOps::managed("TimestampTzMinMaxOps"))
        }
        sql::postgres::SQLOperatorClassKind::TimestampTzMinMaxMultiOps => {
            Some(renderer::IndexOps::managed("TimestampTzMinMaxMultiOps"))
        }
        sql::postgres::SQLOperatorClassKind::TimeBloomOps => Some(renderer::IndexOps::managed("TimeBloomOps")),
        sql::postgres::SQLOperatorClassKind::TimeMinMaxOps => Some(renderer::IndexOps::managed("TimeMinMaxOps")),
        sql::postgres::SQLOperatorClassKind::TimeMinMaxMultiOps => {
            Some(renderer::IndexOps::managed("TimeMinMaxMultiOps"))
        }
        sql::postgres::SQLOperatorClassKind::TimeTzBloomOps => Some(renderer::IndexOps::managed("TimeTzBloomOps")),
        sql::postgres::SQLOperatorClassKind::TimeTzMinMaxOps => Some(renderer::IndexOps::managed("TimeTzMinMaxOps")),
        sql::postgres::SQLOperatorClassKind::TimeTzMinMaxMultiOps => {
            Some(renderer::IndexOps::managed("TimeTzMinMaxMultiOps"))
        }
        sql::postgres::SQLOperatorClassKind::UuidBloomOps => Some(renderer::IndexOps::managed("UuidBloomOps")),
        sql::postgres::SQLOperatorClassKind::UuidMinMaxOps => Some(renderer::IndexOps::managed("UuidMinMaxOps")),
        sql::postgres::SQLOperatorClassKind::UuidMinMaxMultiOps => {
            Some(renderer::IndexOps::managed("UuidMinMaxMultiOps"))
        }
        sql::postgres::SQLOperatorClassKind::Raw(ref c) => Some(renderer::IndexOps::raw(c)),
    }
}
