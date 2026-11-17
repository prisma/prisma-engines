//! Kingbase MySQL schema description.

use crate::{getters::Getter, parsers::Parser, *};
use indexmap::IndexMap;
use psl::{builtin_connectors::KingbaseMySqlType, datamodel_connector::NativeTypeInstance};
use quaint::prelude::{Queryable, ResultRow};
use std::{collections::HashMap, sync::LazyLock};
use tracing::trace;

fn is_mariadb(version: &str) -> bool {
    version.contains("MariaDB")
}

enum Flavour {
    Mysql,
    MariaDb,
}

struct EnumColumn {
    namespace: String,
    name: String,
    variants: Vec<String>,
}

impl Flavour {
    fn from_version(version_string: &str) -> Self {
        if is_mariadb(version_string) {
            Self::MariaDb
        } else {
            Self::Mysql
        }
    }
}

#[enumflags2::bitflags]
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum Circumstances {
    MariaDb,
    MySql56,
    MySql57,
    CheckConstraints,
}

/// Constraint ownership and physical names that are not represented by the
/// MySQL-compatible index metadata.
#[derive(Debug, Default, Clone)]
pub struct KingbaseMysqlSchemaExt {
    /// Physical index identifiers, separate from table-scoped logical names.
    pub physical_index_names: HashMap<IndexId, String>,
    /// The owning unique constraint's name, keyed by its supporting index.
    pub unique_constraint_names: HashMap<IndexId, String>,
    /// The actual primary-key constraint's name, keyed by its supporting index.
    pub primary_key_constraint_names: HashMap<IndexId, String>,
}

pub const INDEX_LOGICAL_NAME_PREFIX: &str = "prisma:logical-index:";

/// Stable, schema-wide physical name for a custom, table-scoped MySQL name.
/// FNV-1a is deliberately fixed rather than Rust's version-dependent hasher.
pub fn physical_index_name(table: &str, logical_name: &str) -> String {
    let hash = table
        .bytes()
        .chain([0])
        .chain(logical_name.bytes())
        .fold(0xcbf29ce484222325u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
        });
    format!("_prisma_idx_{hash:016x}")
}

/// Compare numeric default cast chains after removing only casts that preserve
/// the exact literal. This does not evaluate expressions or rewrite their SQL.
/// Callers must restrict this comparison to numeric columns.
pub fn numeric_default_expressions_match(previous: &str, next: &str) -> bool {
    use sqlparser::{
        ast::{CastKind, DataType, ExactNumberInfo, Expr, UnaryOperator, Value},
        dialect::PostgreSqlDialect,
        parser::Parser as SqlParser,
        tokenizer::Token,
    };

    #[derive(PartialEq)]
    struct NumericExpression {
        literal: bigdecimal::BigDecimal,
        casts: Vec<(u64, i64)>,
        quoted: bool,
    }

    fn normalize(expression: &Expr) -> Option<NumericExpression> {
        match expression {
            Expr::Nested(inner) => normalize(inner),
            Expr::Value(value) => {
                let (literal, quoted) = match &value.value {
                    Value::Number(number, false) => (number, false),
                    Value::SingleQuotedString(number) => (number, true),
                    _ => return None,
                };
                Some(NumericExpression {
                    literal: literal.parse().ok()?,
                    casts: Vec::new(),
                    quoted,
                })
            }
            Expr::UnaryOp {
                op: UnaryOperator::Minus | UnaryOperator::Plus,
                expr,
            } => {
                let mut inner = normalize(expr)?;
                if inner.quoted || !inner.casts.is_empty() {
                    return None;
                }
                if matches!(
                    expression,
                    Expr::UnaryOp {
                        op: UnaryOperator::Minus,
                        ..
                    }
                ) {
                    inner.literal = -inner.literal;
                }
                Some(inner)
            }
            Expr::Cast {
                kind: kind @ (CastKind::DoubleColon | CastKind::Cast),
                expr,
                data_type,
                array: false,
                format: None,
            } => {
                let (DataType::Numeric(info) | DataType::Decimal(info)) = data_type else {
                    return None;
                };
                let mut inner = normalize(expr)?;
                inner.quoted = false;
                let (precision, scale) = match info {
                    // A parameterless MySQL CAST can supply implicit precision.
                    // Only the explicitly sized CAST forms are normalized here.
                    ExactNumberInfo::None if matches!(kind, CastKind::Cast) => return None,
                    ExactNumberInfo::None => return Some(inner),
                    ExactNumberInfo::Precision(p) => (*p, 0),
                    ExactNumberInfo::PrecisionAndScale(p, s) => (*p, *s),
                };
                if !(1..=1000).contains(&precision) || !(0..=1000).contains(&scale) {
                    return None;
                }
                // After a semantic cast, the original literal no longer proves
                // that a later typmod is harmless. Keep the remaining chain.
                if !inner.casts.is_empty()
                    || !SqlSchemaDescriber::numeric_default_fits_cast(
                        &inner.literal,
                        &format!("numeric({precision},{scale})"),
                    )
                {
                    inner.casts.push((precision, scale));
                }
                Some(inner)
            }
            _ => None,
        }
    }

    let parse = |input: &str| {
        let mut parser = SqlParser::new(&PostgreSqlDialect {}).try_with_sql(input).ok()?;
        let expression = parser.parse_expr().ok()?;
        if parser.peek_token().token != Token::EOF {
            return None;
        }
        let normalized = normalize(&expression)?;
        (!normalized.quoted).then_some(normalized)
    };
    match (parse(previous), parse(next)) {
        (Some(previous), Some(next)) => previous == next,
        _ => false,
    }
}

pub struct SqlSchemaDescriber<'a> {
    conn: &'a dyn Queryable,
    circumstances: BitFlags<Circumstances>,
}

#[async_trait::async_trait]
impl super::SqlSchemaDescriberBackend for SqlSchemaDescriber<'_> {
    #[tracing::instrument(skip(self))]
    async fn describe(&self, schemas: &[&str]) -> DescriberResult<SqlSchema> {
        let schema = schemas[0];
        let mut sql_schema = SqlSchema::default();
        let version = self.conn.version().await.ok().flatten();
        let flavour = version
            .as_ref()
            .map(|s| Flavour::from_version(s))
            .unwrap_or(Flavour::Mysql);

        sql_schema.views = self.get_views(schema).await?;

        let table_names = self.get_table_names(schema, &mut sql_schema).await?;
        sql_schema.tables.reserve(table_names.len());
        sql_schema.table_columns.reserve(table_names.len());

        self.get_constraints(schema, &table_names, &mut sql_schema).await?;

        Self::get_all_columns(&table_names, self.conn, schema, &mut sql_schema, &flavour).await?;
        push_foreign_keys(schema, &table_names, &mut sql_schema, self.conn).await?;
        push_indexes(&table_names, schema, &mut sql_schema, self.conn).await?;

        sql_schema.procedures = self.get_procedures(schema).await?;

        Ok(sql_schema)
    }

    #[tracing::instrument(skip(self))]
    async fn version(&self) -> crate::DescriberResult<Option<String>> {
        Ok(self.conn.version().await?)
    }
}

async fn push_indexes(
    table_ids: &IndexMap<String, TableId>,
    schema_name: &str,
    sql_schema: &mut SqlSchema,
    conn: &dyn Queryable,
) -> DescriberResult<()> {
    // Kingbase MySQL does not expose MySQL's information_schema.statistics view.
    // Read the equivalent metadata from Kingbase's native system views.
    let sql = include_str!("kingbase_mysql/indexes_query.sql");

    let rows = conn.query_raw(sql, &[schema_name.into()]).await?;
    push_index_rows(table_ids, sql_schema, rows);
    Ok(())
}

fn push_index_rows(
    table_ids: &IndexMap<String, TableId>,
    sql_schema: &mut SqlSchema,
    rows: quaint::connector::ResultSet,
) {
    // `SqlSchema` walkers expect indexes to be grouped by table id. The
    // catalog query's collation is not guaranteed to match the binary sort
    // used when table ids are assigned, so normalize the order here instead
    // of relying on database-specific collation behavior.
    let mut rows = rows
        .into_iter()
        .filter_map(|row| {
            let table_name = row.get_string("table_name")?;
            let table_id = table_ids.get(table_name.as_str()).copied()?;
            let index_name = row.get_expect_string("index_name");
            let seq_in_index = row.get_expect_i64("seq_in_index");

            Some((table_id, index_name, seq_in_index, row))
        })
        .collect::<Vec<_>>();

    rows.sort_unstable_by(|(table_a, index_a, seq_a, _), (table_b, index_b, seq_b, _)| {
        table_a
            .cmp(table_b)
            .then_with(|| index_a.cmp(index_b))
            .then_with(|| seq_a.cmp(seq_b))
    });
    let mut ext = KingbaseMysqlSchemaExt::default();
    let mut current_index_id: Option<IndexId> = None;
    let mut skip_current_index = false;
    let mut expression_position = 0;

    let remove_last_index = |schema: &mut SqlSchema, ext: &mut KingbaseMysqlSchemaExt, index_id: IndexId| {
        ext.unique_constraint_names.remove(&index_id);
        ext.physical_index_names.remove(&index_id);
        ext.primary_key_constraint_names.remove(&index_id);
        schema.indexes.pop().unwrap();
        loop {
            match schema.index_columns.last() {
                Some(col) if col.index_id == index_id => {
                    schema.index_columns.pop().unwrap();
                }
                None | Some(_) => break,
            }
        }
    };

    for (table_id, _, _, row) in rows {
        let physical_name = row.get_expect_string("index_name");
        let index_name = row
            .get_string("index_description")
            .and_then(|comment| comment.strip_prefix(INDEX_LOGICAL_NAME_PREFIX).map(str::to_owned))
            .filter(|logical| !logical.is_empty() && physical_name.starts_with("_prisma_idx_"))
            .unwrap_or_else(|| physical_name.clone());
        let mut length = row.get_u32("partial");

        let sort_order = row.get_string("column_order").map(|v| match v.as_ref() {
            "A" => SQLSortOrder::Asc,
            "D" => SQLSortOrder::Desc,
            misc => panic!("Unexpected sort order `{misc}`, collation should be A, D or Null"),
        });

        let seq_in_index = row.get_expect_i64("seq_in_index"); // starts at 1

        if seq_in_index == 1 {
            current_index_id = None;
            skip_current_index = false;
            expression_position = 0;
        }

        if seq_in_index == 1
            && row.get_string("index_type").as_deref() == Some("gin")
            && let Some(columns) = fulltext_index_columns(row.get_string("index_expression").as_deref())
        {
            let index_id = sql_schema.push_fulltext_index(table_id, index_name);
            ext.physical_index_names.insert(index_id, physical_name);

            for column_name in columns {
                let column_id = sql_schema
                    .walk(table_id)
                    .column(&column_name)
                    .expect("Kingbase full-text index column was not found in the described table")
                    .id;
                sql_schema.push_index_column(IndexColumn {
                    index_id,
                    column_id,
                    sort_order: None,
                    length: None,
                });
            }

            current_index_id = Some(index_id);
            continue;
        }

        let column_name = row.get_string("column_name").or_else(|| {
            let position = expression_position;
            expression_position += 1;
            if !index_expression_is_prefix(row.get_string("index_expression_nodes").as_deref(), position) {
                return None;
            }
            let (name, prefix_length) = prefix_index_column(row.get_string("index_column_definition").as_deref()?)?;
            let column = sql_schema.walk(table_id).column(&name)?;
            if !matches!(
                column.column_type_family(),
                ColumnTypeFamily::String | ColumnTypeFamily::Binary
            ) {
                return None;
            }
            length = Some(prefix_length);
            Some(name)
        });
        let Some(column_name) = column_name else {
            // Prisma does not model arbitrary expression indexes. Skip the
            // whole index, including any later ordinary columns, rather than
            // attaching those columns to the previous index.
            if let Some(index_id) = current_index_id.take() {
                remove_last_index(sql_schema, &mut ext, index_id);
            }
            skip_current_index = true;
            continue;
        };

        if skip_current_index {
            continue;
        }

        let column_id = if let Some(col) = sql_schema.walk(table_id).column(&column_name) {
            col.id
        } else {
            continue;
        };

        let is_unique = !row.get_expect_bool("non_unique");
        let is_pk = row.get_expect_bool("is_primary");
        let is_fulltext = row.get_string("index_type").as_deref() == Some("FULLTEXT");

        if seq_in_index == 1 {
            // new index!
            let index_id = if is_pk {
                sql_schema.push_primary_key(table_id, String::new())
            } else if is_unique {
                sql_schema.push_unique_constraint(table_id, index_name)
            } else if is_fulltext {
                sql_schema.push_fulltext_index(table_id, index_name)
            } else {
                sql_schema.push_index(table_id, index_name)
            };

            if !is_pk {
                ext.physical_index_names.insert(index_id, physical_name);
            }
            if is_pk {
                ext.primary_key_constraint_names
                    .insert(index_id, row.get_expect_string("constraint_name"));
            } else if is_unique && let Some(name) = row.get_string("constraint_name") {
                ext.unique_constraint_names.insert(index_id, name);
            }

            current_index_id = Some(index_id);
        }

        sql_schema.push_index_column(IndexColumn {
            index_id: current_index_id.unwrap(),
            column_id,
            sort_order,
            length,
        });
    }

    sql_schema.set_connector_data(Box::new(ext));
}

/// A function call such as `abs(8)` can have the same deparsed spelling as a
/// prefix on a column named `abs`. Only R3's PREFIXINDEX nodes are prefixes.
/// Positions count expression keys, excluding ordinary columns in indkey.
fn index_expression_is_prefix(nodes: Option<&str>, position: usize) -> bool {
    let Some(nodes) = nodes else { return false };
    let mut depth = 0_u32;
    let mut quoted = false;
    let mut escaped = false;
    let mut expression = 0;
    for (offset, character) in nodes.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
        } else if character == '"' {
            quoted = !quoted;
        } else if !quoted {
            match character {
                '{' => {
                    if depth == 0 {
                        if expression == position {
                            return nodes[offset + 1..].split_whitespace().next() == Some("PREFIXINDEX");
                        }
                        expression += 1;
                    }
                    depth += 1;
                }
                '}' => match depth.checked_sub(1) {
                    Some(next) => depth = next,
                    None => return false,
                },
                _ => (),
            }
        }
    }
    false
}

fn prefix_index_column(definition: &str) -> Option<(String, u32)> {
    let (identifier, length) = definition.strip_suffix(')')?.rsplit_once('(')?;
    let length: u32 = length.parse().ok()?;
    if length == 0 {
        return None;
    }
    let name = if let Some(quoted) = identifier.strip_prefix('"') {
        let mut characters = quoted.strip_suffix('"')?.chars().peekable();
        let mut name = String::new();
        while let Some(character) = characters.next() {
            if character == '"' && characters.next() != Some('"') {
                return None;
            }
            name.push(character);
        }
        name
    } else {
        if identifier.is_empty() || !identifier.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$') {
            return None;
        }
        identifier.to_owned()
    };
    Some((name, length))
}

fn fulltext_index_columns(index_expression: Option<&str>) -> Option<Vec<String>> {
    let tokens = tokenize_fulltext_expression(index_expression?)?;
    FulltextExpressionParser::new(&tokens).parse()
}

/// Parses only the `to_tsvector('simple', textcat(COALESCE(...), ...))`
/// expression rendered by `KingbaseMysqlRenderer`. A GIN index that merely
/// happens to use `to_tsvector` must remain an expression index during
/// introspection rather than being reinterpreted as Prisma `@@fulltext`.
#[derive(Debug, PartialEq, Eq)]
enum FulltextToken {
    Identifier(String),
    StringLiteral(String),
    LeftParen,
    RightParen,
    Comma,
    Cast,
}

fn tokenize_fulltext_expression(input: &str) -> Option<Vec<FulltextToken>> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(character) = chars.next() {
        match character {
            character if character.is_ascii_whitespace() => (),
            '(' => tokens.push(FulltextToken::LeftParen),
            ')' => tokens.push(FulltextToken::RightParen),
            ',' => tokens.push(FulltextToken::Comma),
            ':' if chars.next_if_eq(&':').is_some() => tokens.push(FulltextToken::Cast),
            '"' => {
                let mut identifier = String::new();

                loop {
                    match chars.next()? {
                        '"' if chars.next_if_eq(&'"').is_some() => identifier.push('"'),
                        '"' => break,
                        character => identifier.push(character),
                    }
                }

                tokens.push(FulltextToken::Identifier(identifier));
            }
            '\'' => {
                let mut value = String::new();

                loop {
                    match chars.next()? {
                        '\'' if chars.next_if_eq(&'\'').is_some() => value.push('\''),
                        '\'' => break,
                        character => value.push(character),
                    }
                }

                tokens.push(FulltextToken::StringLiteral(value));
            }
            character if character.is_ascii_alphabetic() || character == '_' => {
                let mut identifier = String::from(character);
                while matches!(chars.peek(), Some(character) if character.is_ascii_alphanumeric() || *character == '_' || *character == '$')
                {
                    identifier.push(chars.next().expect("peeked character must be available"));
                }
                tokens.push(FulltextToken::Identifier(identifier));
            }
            _ => return None,
        }
    }

    Some(tokens)
}

struct FulltextExpressionParser<'a> {
    tokens: &'a [FulltextToken],
    cursor: usize,
}

impl<'a> FulltextExpressionParser<'a> {
    fn new(tokens: &'a [FulltextToken]) -> Self {
        Self { tokens, cursor: 0 }
    }

    fn parse(mut self) -> Option<Vec<String>> {
        self.consume_identifier("to_tsvector")?;
        self.consume(FulltextToken::LeftParen)?;
        self.consume_string("simple")?;
        self.consume_casts()?;
        self.consume(FulltextToken::Comma)?;
        let columns = self.parse_document()?;
        self.consume(FulltextToken::RightParen)?;
        self.consume_casts()?;

        (self.cursor == self.tokens.len()).then_some(columns)
    }

    fn parse_document(&mut self) -> Option<Vec<String>> {
        let parentheses = self.consume_open_parentheses();

        let columns = if self.next_identifier_is("coalesce") {
            vec![self.parse_coalesce()?]
        } else {
            self.consume_identifier("textcat")?;
            self.consume(FulltextToken::LeftParen)?;
            let mut columns = vec![self.parse_coalesce()?];
            self.consume(FulltextToken::Comma)?;
            self.consume_identifier("textcat")?;
            self.consume(FulltextToken::LeftParen)?;
            self.consume_string(" ")?;
            self.consume_casts()?;
            self.consume(FulltextToken::Comma)?;
            columns.extend(self.parse_document()?);
            self.consume(FulltextToken::RightParen)?;
            self.consume(FulltextToken::RightParen)?;
            columns
        };

        for _ in 0..parentheses {
            self.consume(FulltextToken::RightParen)?;
        }
        self.consume_casts()?;

        Some(columns)
    }

    fn parse_coalesce(&mut self) -> Option<String> {
        let parentheses = self.consume_open_parentheses();
        self.consume_identifier("coalesce")?;
        self.consume(FulltextToken::LeftParen)?;
        let column = self.consume_any_identifier()?;
        self.consume(FulltextToken::Comma)?;
        self.consume_string("")?;
        self.consume_casts()?;
        self.consume(FulltextToken::RightParen)?;

        for _ in 0..parentheses {
            self.consume(FulltextToken::RightParen)?;
        }
        self.consume_casts()?;

        Some(column)
    }

    fn consume_open_parentheses(&mut self) -> usize {
        let start = self.cursor;
        while self.consume(FulltextToken::LeftParen).is_some() {}
        self.cursor - start
    }

    fn consume_casts(&mut self) -> Option<()> {
        while self.consume(FulltextToken::Cast).is_some() {
            let cast_type = self.consume_any_identifier()?;
            if cast_type.eq_ignore_ascii_case("character") {
                self.consume_identifier("varying")?;
            }
        }

        Some(())
    }

    fn consume_any_identifier(&mut self) -> Option<String> {
        let FulltextToken::Identifier(identifier) = self.tokens.get(self.cursor)? else {
            return None;
        };
        self.cursor += 1;
        Some(identifier.clone())
    }

    fn next_identifier_is(&self, expected: &str) -> bool {
        matches!(self.tokens.get(self.cursor), Some(FulltextToken::Identifier(identifier)) if identifier.eq_ignore_ascii_case(expected))
    }

    fn consume_identifier(&mut self, expected: &str) -> Option<()> {
        self.next_identifier_is(expected).then(|| {
            self.cursor += 1;
        })
    }

    fn consume_string(&mut self, expected: &str) -> Option<()> {
        matches!(self.tokens.get(self.cursor), Some(FulltextToken::StringLiteral(value)) if value == expected).then(
            || {
                self.cursor += 1;
            },
        )
    }

    fn consume(&mut self, expected: FulltextToken) -> Option<()> {
        (self.tokens.get(self.cursor) == Some(&expected)).then(|| {
            self.cursor += 1;
        })
    }
}

impl Parser for SqlSchemaDescriber<'_> {}

impl<'a> SqlSchemaDescriber<'a> {
    /// Constructor.
    pub fn new(conn: &'a dyn Queryable, circumstances: BitFlags<Circumstances>) -> SqlSchemaDescriber<'a> {
        SqlSchemaDescriber { conn, circumstances }
    }

    #[tracing::instrument(skip(self))]
    async fn get_views(&self, schema: &str) -> DescriberResult<Vec<View>> {
        let sql = include_str!("kingbase_mysql/views_query.sql");

        let result_set = self.conn.query_raw(sql, &[schema.into()]).await?;
        let mut views = Vec::with_capacity(result_set.len());

        for row in result_set.into_iter() {
            views.push(View {
                namespace_id: NamespaceId(0),
                name: row.get_expect_string("view_name"),
                definition: row.get_string("view_sql"),
                description: None,
            })
        }

        Ok(views)
    }

    #[tracing::instrument(skip(self))]
    async fn get_procedures(&self, schema: &str) -> DescriberResult<Vec<Procedure>> {
        let sql = include_str!("kingbase_mysql/procedures_query.sql");

        let rows = self.conn.query_raw(sql, &[schema.into()]).await?;
        let mut procedures = Vec::with_capacity(rows.len());

        for row in rows.into_iter() {
            procedures.push(Procedure {
                namespace_id: NamespaceId(0),
                name: row.get_expect_string("name"),
                definition: row.get_string("definition"),
            });
        }

        Ok(procedures)
    }

    #[tracing::instrument(skip(self))]
    async fn get_table_names(
        &self,
        schema: &str,
        sql_schema: &mut SqlSchema,
    ) -> DescriberResult<IndexMap<String, TableId>> {
        // Only consider tables for which we can read at least one column.
        let sql = include_str!("kingbase_mysql/tables_query.sql");
        let rows = self.conn.query_raw(sql, &[schema.into(), schema.into()]).await?;
        let names = rows.into_iter().map(|row| {
            (
                row.get_expect_string("table_name"),
                row.get_string("create_options")
                    .filter(|c| c.as_str() == "partitioned")
                    .is_some(),
                row.get_string("table_comment").filter(|c| !c.is_empty()),
            )
        });

        let mut map = IndexMap::default();

        for (name, is_partition, description) in names {
            let cloned_name = name.clone();
            let id = if is_partition {
                sql_schema.push_table_with_properties(
                    name,
                    Default::default(),
                    Into::into(TableProperties::IsPartition),
                    description,
                )
            } else {
                sql_schema.push_table(name, Default::default(), description)
            };
            map.insert(cloned_name, id);
        }

        trace!("Found table names: {map:?}");

        Ok(map)
    }

    async fn get_all_columns(
        table_ids: &IndexMap<String, TableId>,
        conn: &dyn Queryable,
        schema_name: &str,
        sql_schema: &mut SqlSchema,
        flavour: &Flavour,
    ) -> DescriberResult<()> {
        let user_defined_enum_variants = Self::get_user_defined_enum_variants(conn, schema_name).await?;
        // We alias all the columns because MySQL column names are case-insensitive in queries, but the
        // information schema column names became upper-case in MySQL 8, causing the code fetching
        // the result values by column name below to fail.
        let sql = include_str!("kingbase_mysql/columns_query.sql");

        let mut table_defaults = Vec::new();
        let mut view_defaults = Vec::new();
        let rows = conn.query_raw(sql, &[schema_name.into()]).await?;

        for col in rows {
            trace!("Got column: {col:?}");
            let table_name = col.get_expect_string("table_name");

            let table_id = table_ids.get(table_name.as_str());
            let view_id = sql_schema.view_walker(table_name.as_str());

            let container_id = match (table_id, view_id) {
                (Some(id), _) => Either::Left(id),
                (_, Some(v_walker)) => Either::Right(v_walker.id),
                (None, None) => continue, // we only care about columns in tables we have access to
            };

            let name = col.get_expect_string("column_name");
            let data_type = col.get_expect_string("data_type");
            let full_data_type = col.get_expect_string("full_data_type");

            let is_nullable = col.get_expect_string("is_nullable").to_lowercase();
            let is_required = match is_nullable.as_ref() {
                "no" => true,
                "yes" => false,
                x => panic!("unrecognized is_nullable variant '{x}'"),
            };

            let arity = if is_required {
                ColumnArity::Required
            } else {
                ColumnArity::Nullable
            };

            let character_maximum_length = col.get_u32("character_maximum_length");
            let time_precision = col.get_u32("datetime_precision");
            let numeric_precision = col.get_u32("numeric_precision");
            let numeric_scale = col.get_u32("numeric_scale");

            let precision = Precision {
                character_maximum_length,
                numeric_precision,
                numeric_scale,
                time_precision,
            };

            let default_value = col.get("column_default");
            let enum_column = user_defined_enum_variants.get(&(table_name.clone(), name.clone()));

            let tpe = Self::get_column_type(
                (&table_name, &name),
                (&data_type, &full_data_type),
                precision,
                arity,
                sql_schema,
                enum_column,
            );
            let extra = col.get_string("extra").unwrap_or_default().to_lowercase();
            let auto_increment = matches!(extra.as_str(), "auto_increment");

            let default = match default_value {
                None => None,
                Some(param_value) => match param_value.to_string() {
                    None => None,
                    Some(x) if x == "NULL" => None,
                    Some(default_string) => {
                        let default_generated = matches!(extra.as_str(), "default_generated");
                        let maria_db = matches!(flavour, Flavour::MariaDb);
                        let default_expression = default_generated || maria_db;

                        Some(match &tpe.family {
                            ColumnTypeFamily::Int => match Self::parse_int_default(&default_string) {
                                Some(int_value) => DefaultValue::value(int_value),
                                None if default_expression => Self::dbgenerated_expression(&default_string),
                                None => DefaultValue::db_generated(default_string),
                            },
                            ColumnTypeFamily::BigInt => match Self::parse_big_int_default(&default_string) {
                                Some(int_value) => DefaultValue::value(int_value),
                                None if default_expression => Self::dbgenerated_expression(&default_string),
                                None => DefaultValue::db_generated(default_string),
                            },
                            ColumnTypeFamily::Float | ColumnTypeFamily::Decimal => match Self::parse_float_default(
                                &default_string,
                                tpe.native_type
                                    .as_ref()
                                    .map(|native| native.downcast_ref::<KingbaseMySqlType>()),
                            ) {
                                Some(float_value) => DefaultValue::value(float_value),
                                None if default_expression => Self::dbgenerated_expression(&default_string),
                                None => DefaultValue::db_generated(default_string),
                            },
                            ColumnTypeFamily::Boolean => match default_string.as_str() {
                                "true" => DefaultValue::value(true),
                                "false" => DefaultValue::value(false),
                                _ => match Self::parse_int(&default_string) {
                                    Some(PrismaValue::Int(1)) => DefaultValue::value(true),
                                    Some(PrismaValue::Int(0)) => DefaultValue::value(false),
                                    _ if default_expression => Self::dbgenerated_expression(&default_string),
                                    _ => DefaultValue::db_generated(default_string),
                                },
                            },
                            ColumnTypeFamily::String => {
                                if default_generated
                                    || (maria_db && !matches!(default_string.chars().next(), Some('\'')))
                                {
                                    Self::dbgenerated_expression(&default_string)
                                } else if let Some(literal) = Self::string_default_literal(
                                    &default_string,
                                    tpe.native_type
                                        .as_ref()
                                        .map(|native| native.downcast_ref::<KingbaseMySqlType>()),
                                ) {
                                    DefaultValue::value(PrismaValue::String(Self::unquote_default_literal(literal)))
                                } else {
                                    // Length-qualified and unknown casts can truncate or
                                    // otherwise change the literal. Keep the server expression.
                                    DefaultValue::db_generated(default_string)
                                }
                            }
                            ColumnTypeFamily::DateTime => Self::temporal_default(
                                &default_string,
                                tpe.native_type
                                    .as_ref()
                                    .map(|native| native.downcast_ref::<KingbaseMySqlType>()),
                                col.get_bool("default_has_explicit_cast"),
                            ),
                            ColumnTypeFamily::Binary => match default_expression {
                                true => Self::dbgenerated_expression(&default_string),
                                false => DefaultValue::db_generated(default_string),
                            },
                            ColumnTypeFamily::Json => match default_expression {
                                true => Self::dbgenerated_expression(&default_string),
                                false => DefaultValue::db_generated(default_string),
                            },
                            ColumnTypeFamily::Uuid => match default_expression {
                                true => Self::dbgenerated_expression(&default_string),
                                false => DefaultValue::db_generated(default_string),
                            },
                            ColumnTypeFamily::Enum(_) => {
                                if default_generated
                                    || (maria_db && !matches!(default_string.chars().next(), Some('\'')))
                                {
                                    Self::dbgenerated_expression(&default_string)
                                } else if let Some(literal) = Self::enum_default_literal(
                                    &default_string,
                                    enum_column.map_or(schema_name, |column| column.namespace.as_str()),
                                    enum_column.map_or(full_data_type.as_str(), |column| column.name.as_str()),
                                ) {
                                    DefaultValue::value(PrismaValue::Enum(Self::unquote_default_literal(literal)))
                                } else {
                                    DefaultValue::db_generated(default_string)
                                }
                            }
                            ColumnTypeFamily::Udt(_) | ColumnTypeFamily::Unsupported(_) => match default_expression {
                                true => Self::dbgenerated_expression(&default_string),
                                false => DefaultValue::db_generated(default_string),
                            },
                        })
                    }
                },
            };

            match container_id {
                Either::Left(table_id) => {
                    table_defaults.push((table_id, default));
                }
                Either::Right(view_id) => {
                    view_defaults.push((view_id, default));
                }
            }

            let description = col.get_string("column_comment");

            let col = Column {
                name,
                tpe,
                auto_increment,
                description,
            };

            match container_id {
                Either::Left(table_id) => {
                    sql_schema.table_columns.push((*table_id, col));
                }
                Either::Right(view_id) => {
                    sql_schema.view_columns.push((view_id, col));
                }
            }
        }

        sql_schema.table_columns.sort_by_key(|(table_id, _)| *table_id);
        sql_schema.view_columns.sort_by_key(|(view_id, _)| *view_id);

        table_defaults.sort_by_key(|(table_id, _)| *table_id);
        view_defaults.sort_by_key(|(view_id, _)| *view_id);

        for (i, (_, default)) in table_defaults.into_iter().enumerate() {
            if let Some(default) = default {
                sql_schema.push_table_default_value(TableColumnId(i as u32), default);
            }
        }

        for (i, (_, default)) in view_defaults.into_iter().enumerate() {
            if let Some(default) = default {
                sql_schema.push_view_default_value(ViewColumnId(i as u32), default);
            }
        }

        Ok(())
    }

    async fn get_user_defined_enum_variants(
        conn: &dyn Queryable,
        schema_name: &str,
    ) -> DescriberResult<HashMap<(String, String), EnumColumn>> {
        // Kingbase exposes MySQL ENUM columns through user-defined
        // types. The information_schema COLUMNS view only contains the generated type name,
        // so follow each column's type OID to sys_enum, independently of search_path.
        let sql = include_str!("kingbase_mysql/enums_query.sql");

        let rows = conn.query_raw(sql, &[schema_name.into()]).await?;
        let mut variants = HashMap::new();

        for row in rows {
            let table_name = row.get_expect_string("table_name");
            let column_name = row.get_expect_string("column_name");
            let enum_label = row.get_expect_string("enum_label");
            variants
                .entry((table_name, column_name))
                .or_insert_with(|| EnumColumn {
                    namespace: row.get_expect_string("type_namespace"),
                    name: row.get_expect_string("type_name"),
                    variants: Vec::new(),
                })
                .variants
                .push(enum_label);
        }

        Ok(variants)
    }

    fn dbgenerated_expression(default_string: &str) -> DefaultValue {
        if matches!(default_string.chars().next(), Some('(')) {
            DefaultValue::db_generated(default_string.to_owned())
        } else {
            let mut introspected_default = String::with_capacity(default_string.len());
            introspected_default.push('(');
            introspected_default.push_str(default_string);
            introspected_default.push(')');
            DefaultValue::db_generated(introspected_default)
        }
    }

    fn parse_int_default(value: &str) -> Option<PrismaValue> {
        let value = value
            .split_once("::")
            .map_or(value, |(value, _)| value)
            .trim_matches('\'');

        Self::parse_int(value)
    }

    /// Only normalize casts that preserve a numeric literal at the column's
    /// precision. KES also decorates ordinary numeric literals with typmods;
    /// normalize those only when the literal fits without rounding or overflow.
    fn parse_float_default(value: &str, native_type: Option<&KingbaseMySqlType>) -> Option<PrismaValue> {
        let value = value.trim();
        let (literal, suffix) = match Self::quoted_default_parts(value) {
            Some(parts) => parts,
            None if value.contains('\'') => return None,
            None => value.find("::").map_or((value, ""), |offset| value.split_at(offset)),
        };
        let parsed = Self::parse_float(literal.trim())?;
        let PrismaValue::Float(number) = &parsed else {
            return None;
        };
        let mut floating_cast = false;
        if !suffix.is_empty() {
            let casts = suffix.strip_prefix("::")?;
            for cast in casts.split("::") {
                let cast = cast.trim().to_ascii_lowercase();
                let cast = cast
                    .strip_prefix("pg_catalog.")
                    .or_else(|| cast.strip_prefix("sys_catalog."))
                    .unwrap_or(&cast);
                match cast {
                    "numeric" | "decimal" => (),
                    "real" if matches!(native_type, Some(KingbaseMySqlType::Float)) => floating_cast = true,
                    "double precision" if matches!(native_type, Some(KingbaseMySqlType::Double)) => {
                        floating_cast = true
                    }
                    _ if !floating_cast && Self::numeric_default_fits_cast(number, cast) => (),
                    _ => return None,
                }
            }
        }
        Some(parsed)
    }

    fn numeric_default_fits_cast(number: &bigdecimal::BigDecimal, cast: &str) -> bool {
        let Some((name, parameters)) = cast.split_once('(') else {
            return false;
        };
        if !matches!(name.trim(), "numeric" | "decimal") {
            return false;
        }
        let Some(parameters) = parameters.strip_suffix(')') else {
            return false;
        };
        let (precision, scale) = parameters.split_once(',').unwrap_or((parameters, "0"));
        let (Ok(precision), Ok(scale)) = (precision.trim().parse::<i64>(), scale.trim().parse::<i64>()) else {
            return false;
        };
        if !(1..=1000).contains(&precision) || !(0..=1000).contains(&scale) {
            return false;
        }
        let (coefficient, fractional_digits) = number.normalized().as_bigint_and_exponent();
        let coefficient = coefficient.to_string();
        if coefficient == "0" {
            return true;
        }
        let digits = coefficient.trim_start_matches('-').len() as i64;
        let Some(integer_digits) = digits.checked_sub(fractional_digits) else {
            return false;
        };
        fractional_digits <= scale && integer_digits.max(0) <= (precision - scale).max(0)
    }

    fn parse_big_int_default(value: &str) -> Option<PrismaValue> {
        let value = value
            .split_once("::")
            .map_or(value, |(value, _)| value)
            .trim_matches('\'');

        Self::parse_big_int(value)
    }

    fn get_column_type(
        (table, column_name): (&str, &str),
        (data_type, full_data_type): (&str, &str),
        precision: Precision,
        arity: ColumnArity,
        sql_schema: &mut SqlSchema,
        enum_column: Option<&EnumColumn>,
    ) -> ColumnType {
        static UNSIGNEDNESS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new("(?i)unsigned$").unwrap());

        // Kingbase's MySQL-compatible COLUMNS view may include the precision in DATA_TYPE
        // itself (for example `numeric(10,2)`), whereas MySQL reports the bare type name.
        let data_type = data_type
            .split_once('(')
            .map_or(data_type, |(name, _)| name)
            .trim()
            .to_ascii_lowercase();
        let (family, native_type) = match data_type.as_str() {
            // R1 reports catalog names, while R3 reports MySQL unsigned names.
            "uint4" | "int unsigned" | "integer unsigned" => {
                (ColumnTypeFamily::Int, Some(KingbaseMySqlType::UnsignedInt))
            }
            "uint8" | "bigint unsigned" => (ColumnTypeFamily::BigInt, Some(KingbaseMySqlType::UnsignedBigInt)),
            "tinyint unsigned" => (ColumnTypeFamily::Int, Some(KingbaseMySqlType::UnsignedTinyInt)),
            "smallint unsigned" => (ColumnTypeFamily::Int, Some(KingbaseMySqlType::UnsignedSmallInt)),
            "mediumint unsigned" => (ColumnTypeFamily::Int, Some(KingbaseMySqlType::UnsignedMediumInt)),
            "int" | "integer" if UNSIGNEDNESS_RE.is_match(full_data_type) => {
                (ColumnTypeFamily::Int, Some(KingbaseMySqlType::UnsignedInt))
            }
            "int" | "integer" => (ColumnTypeFamily::Int, Some(KingbaseMySqlType::Int)),
            "smallint" if UNSIGNEDNESS_RE.is_match(full_data_type) => {
                (ColumnTypeFamily::Int, Some(KingbaseMySqlType::UnsignedSmallInt))
            }
            "smallint" => (ColumnTypeFamily::Int, Some(KingbaseMySqlType::SmallInt)),
            "tinyint" if UNSIGNEDNESS_RE.is_match(full_data_type) => {
                (ColumnTypeFamily::Int, Some(KingbaseMySqlType::UnsignedTinyInt))
            }
            "tinyint" => (ColumnTypeFamily::Int, Some(KingbaseMySqlType::TinyInt)),
            "mediumint" if UNSIGNEDNESS_RE.is_match(full_data_type) => {
                (ColumnTypeFamily::Int, Some(KingbaseMySqlType::UnsignedMediumInt))
            }
            "mediumint" => (ColumnTypeFamily::Int, Some(KingbaseMySqlType::MediumInt)),
            "bigint" if UNSIGNEDNESS_RE.is_match(full_data_type) => {
                (ColumnTypeFamily::BigInt, Some(KingbaseMySqlType::UnsignedBigInt))
            }
            "bigint" => (ColumnTypeFamily::BigInt, Some(KingbaseMySqlType::BigInt)),
            "decimal" | "numeric" | "number" => (
                ColumnTypeFamily::Decimal,
                Some(KingbaseMySqlType::Decimal(Some((
                    precision.numeric_precision.unwrap(),
                    precision.numeric_scale.unwrap(),
                )))),
            ),
            "boolean" => (ColumnTypeFamily::Boolean, None),
            "float" => (ColumnTypeFamily::Float, Some(KingbaseMySqlType::Float)),
            "float4" => (ColumnTypeFamily::Float, Some(KingbaseMySqlType::Float)),
            "double" => (ColumnTypeFamily::Float, Some(KingbaseMySqlType::Double)),
            "double precision" => (ColumnTypeFamily::Float, Some(KingbaseMySqlType::Double)),

            "char" => (
                ColumnTypeFamily::String,
                Some(KingbaseMySqlType::Char(
                    Self::extract_precision(full_data_type)
                        .or(precision.character_maximum_length)
                        .unwrap(),
                )),
            ),
            "character" => (
                ColumnTypeFamily::String,
                Some(KingbaseMySqlType::Char(
                    Self::extract_precision(full_data_type)
                        .or(precision.character_maximum_length)
                        .unwrap(),
                )),
            ),
            "bpchar" => (
                ColumnTypeFamily::String,
                Some(KingbaseMySqlType::Char(
                    Self::extract_precision(full_data_type)
                        .or(precision.character_maximum_length)
                        .unwrap(),
                )),
            ),
            "varchar" => (
                ColumnTypeFamily::String,
                Some(KingbaseMySqlType::VarChar(
                    Self::extract_precision(full_data_type)
                        .or(precision.character_maximum_length)
                        .unwrap(),
                )),
            ),
            "text" => (ColumnTypeFamily::String, Some(KingbaseMySqlType::Text)),
            "tinytext" => (ColumnTypeFamily::String, Some(KingbaseMySqlType::TinyText)),
            "mediumtext" => (ColumnTypeFamily::String, Some(KingbaseMySqlType::MediumText)),
            "longtext" => (ColumnTypeFamily::String, Some(KingbaseMySqlType::LongText)),
            "enum" => {
                let enum_name = format!("{table}_{column_name}");
                let enum_id = sql_schema.push_enum(Default::default(), enum_name, None);
                push_enum_variants(full_data_type, enum_id, sql_schema);
                (ColumnTypeFamily::Enum(enum_id), None)
            }
            "user-defined" => match enum_column {
                Some(enum_column) => {
                    let enum_name = format!("{table}_{column_name}");
                    let enum_id = sql_schema.push_enum(Default::default(), enum_name, None);
                    for variant in &enum_column.variants {
                        sql_schema.push_enum_variant(enum_id, variant.clone());
                    }
                    (ColumnTypeFamily::Enum(enum_id), None)
                }
                None if full_data_type.starts_with("Set_") => (ColumnTypeFamily::String, None),
                None => (ColumnTypeFamily::Unsupported(full_data_type.into()), None),
            },
            "json" => (ColumnTypeFamily::Json, Some(KingbaseMySqlType::Json)),
            "set" => (ColumnTypeFamily::String, None),
            //temporal
            "date" => (ColumnTypeFamily::DateTime, Some(KingbaseMySqlType::Date)),
            "time" => (
                //Fixme this can either be a time or a duration -.-
                ColumnTypeFamily::DateTime,
                Some(KingbaseMySqlType::Time(precision.time_precision)),
            ),
            "datetime" => (
                ColumnTypeFamily::DateTime,
                Some(KingbaseMySqlType::DateTime(precision.time_precision)),
            ),
            "timestamp" => (
                ColumnTypeFamily::DateTime,
                Some(KingbaseMySqlType::Timestamp(precision.time_precision)),
            ),
            "year" => (ColumnTypeFamily::Int, Some(KingbaseMySqlType::Year)),
            "bit" if Self::extract_precision(full_data_type).or(precision.numeric_precision) == Some(1) => (
                ColumnTypeFamily::Boolean,
                Some(KingbaseMySqlType::Bit(
                    Self::extract_precision(full_data_type)
                        .or(precision.numeric_precision)
                        .unwrap_or(1),
                )),
            ),
            //01100010 01101001 01110100 01110011 00100110 01100010 01111001 01110100 01100101 01110011 00001010
            "bit" => (
                ColumnTypeFamily::Binary,
                Some(KingbaseMySqlType::Bit(
                    Self::extract_precision(full_data_type)
                        .or(precision.numeric_precision)
                        .unwrap_or(1),
                )),
            ),
            "binary" => (
                ColumnTypeFamily::Binary,
                Some(KingbaseMySqlType::Binary(
                    Self::extract_precision(full_data_type)
                        .or(precision.character_maximum_length)
                        .unwrap(),
                )),
            ),
            "varbinary" => (
                ColumnTypeFamily::Binary,
                Some(KingbaseMySqlType::VarBinary(
                    Self::extract_precision(full_data_type)
                        .or(precision.character_maximum_length)
                        .unwrap(),
                )),
            ),
            "blob" => (ColumnTypeFamily::Binary, Some(KingbaseMySqlType::Blob)),
            "tinyblob" => (ColumnTypeFamily::Binary, Some(KingbaseMySqlType::TinyBlob)),
            "mediumblob" => (ColumnTypeFamily::Binary, Some(KingbaseMySqlType::MediumBlob)),
            "longblob" => (ColumnTypeFamily::Binary, Some(KingbaseMySqlType::LongBlob)),
            //spatial
            "geometry" => (ColumnTypeFamily::Unsupported(full_data_type.into()), None),
            "point" => (ColumnTypeFamily::Unsupported(full_data_type.into()), None),
            "linestring" => (ColumnTypeFamily::Unsupported(full_data_type.into()), None),
            "polygon" => (ColumnTypeFamily::Unsupported(full_data_type.into()), None),
            "multipoint" => (ColumnTypeFamily::Unsupported(full_data_type.into()), None),
            "multilinestring" => (ColumnTypeFamily::Unsupported(full_data_type.into()), None),
            "multipolygon" => (ColumnTypeFamily::Unsupported(full_data_type.into()), None),
            "geometrycollection" => (ColumnTypeFamily::Unsupported(full_data_type.into()), None),
            _ => (ColumnTypeFamily::Unsupported(full_data_type.into()), None),
        };

        ColumnType {
            full_data_type: full_data_type.to_owned(),
            family,
            arity,
            native_type: native_type.map(NativeTypeInstance::new::<KingbaseMySqlType>),
        }
    }

    /// Return the constraints that are not primary keys, foreign keys, or unique keys, fulltext, or spacial.
    /// Namely, this currently just returns CHECK constraints.
    async fn get_constraints(
        &self,
        schema_name: &str,
        table_names: &IndexMap<String, TableId>,
        sql_schema: &mut SqlSchema,
    ) -> DescriberResult<()> {
        if !self.supports_check_constraints() {
            return Ok(());
        }

        let sql = include_str!("kingbase_mysql/constraints_query.sql");

        let rows = self.conn.query_raw(sql, &[schema_name.into()]).await?;

        for row in rows {
            let table_name = row.get_expect_string("table_name");
            let constraint_name = row.get_expect_string("constraint_name");
            let constraint_type = row.get_expect_string("constraint_type");

            let table_id = match table_names.get(&table_name) {
                Some(id) => *id,
                None => continue,
            };

            if constraint_type.as_str() == "check" {
                sql_schema.check_constraints.push((table_id, constraint_name));
            }
        }

        sql_schema.check_constraints.sort_by_key(|(id, _)| *id);

        Ok(())
    }

    fn extract_precision(input: &str) -> Option<u32> {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r".*\(([0-9]+)(?:,[0-9]+)?\)").unwrap());
        RE.captures(input)
            .and_then(|cap| cap.get(1).map(|precision| precision.as_str().parse::<u32>().unwrap()))
    }

    /// Only remove typmod-free character casts whose literal representation is
    /// unchanged. Scan the quoted literal first: `::` inside its contents is data.
    fn string_default_literal<'d>(default: &'d str, target_native: Option<&KingbaseMySqlType>) -> Option<&'d str> {
        let (literal, suffix) = Self::quoted_default_parts(default)?;
        if suffix.is_empty() {
            return Some(literal);
        }
        let casts = suffix.strip_prefix("::")?;
        casts
            .split("::")
            .all(|cast| {
                // Converting bpchar to varchar/text strips trailing
                // spaces; normalize it only for a CHAR target.
                matches!(
                    cast.trim().to_ascii_lowercase().as_str(),
                    "text" | "varchar" | "character varying"
                ) || (cast.trim().eq_ignore_ascii_case("bpchar")
                    && matches!(target_native, Some(KingbaseMySqlType::Char(_))))
            })
            .then_some(literal)
    }

    fn enum_default_literal<'d>(default: &'d str, namespace: &str, native_type: &str) -> Option<&'d str> {
        let (literal, mut suffix) = Self::quoted_default_parts(default)?;
        let quote = |name: &str| format!("\"{}\"", name.replace('"', "\"\""));
        let mut casts = vec!["text".to_owned(), "varchar".to_owned(), "character varying".to_owned()];
        for name in [native_type.to_owned(), quote(native_type)] {
            casts.push(format!("{namespace}.{name}"));
            casts.push(format!("{}.{name}", quote(namespace)));
            casts.push(name);
        }
        // Match whole type spellings so quoted namespaces containing `::`
        // are not split and length-qualified or unrelated casts stay SQL.
        while !suffix.is_empty() {
            let rest = suffix.strip_prefix("::")?.trim_start();
            suffix = casts.iter().find_map(|cast| {
                rest.strip_prefix(cast.as_str())
                    .map(str::trim)
                    .filter(|tail| tail.is_empty() || tail.starts_with("::"))
            })?;
        }
        Some(literal)
    }

    /// KES returns decompiled SQL, not MySQL's unquoted COLUMN_DEFAULT text.
    /// Only a complete quoted literal can be interpreted as a scalar default.
    fn quoted_default_parts(default: &str) -> Option<(&str, &str)> {
        let default = default.trim();
        if !default.starts_with('\'') {
            return None;
        }
        let bytes = default.as_bytes();
        let mut index = 1;
        while index < bytes.len() {
            if bytes[index] != b'\'' {
                index += 1;
            } else if bytes.get(index + 1) == Some(&b'\'') {
                index += 2;
            } else {
                return Some((&default[..=index], default[index + 1..].trim()));
            }
        }
        None
    }

    /// Decode a verified ordinary SQL literal once. Backslashes in KES's
    /// ordinary decompiled literals are data, not MySQL escape sequences.
    fn unquote_default_literal(literal: &str) -> String {
        literal[1..literal.len() - 1].replace("''", "'")
    }

    fn temporal_default(
        value: &str,
        native_type: Option<&KingbaseMySqlType>,
        has_explicit_cast: Option<bool>,
    ) -> DefaultValue {
        if Self::default_is_current_timestamp(value) {
            return DefaultValue::now();
        }

        // Deparsed SQL alone cannot distinguish an implicit TIME assignment
        // from an explicit TIME(0). Only remove decoration when the stored
        // expression confirms there is no explicit cast; missing metadata stays SQL.
        if has_explicit_cast == Some(false)
            && let Some((literal, suffix)) = Self::quoted_default_parts(value)
        {
            let cast = match native_type {
                Some(KingbaseMySqlType::Date) => "::date",
                Some(KingbaseMySqlType::Time(_)) => "::time",
                Some(KingbaseMySqlType::Timestamp(_)) => "::timestamp",
                Some(KingbaseMySqlType::DateTime(_)) => "::datetime",
                _ => "",
            };
            if suffix == cast {
                return DefaultValue::db_generated(literal.to_owned());
            }
        }

        // Keep functions and semantic casts as SQL, including now(): on R3
        // now() has second precision, unlike CURRENT_TIMESTAMP(3).
        DefaultValue::db_generated(value.to_owned())
    }

    /// Tests whether an introspected default value should be categorized as current_timestamp.
    fn default_is_current_timestamp(default_str: &str) -> bool {
        static MYSQL_CURRENT_TIMESTAMP_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?i)^current_timestamp(\([0-9]*\))?$").unwrap());

        MYSQL_CURRENT_TIMESTAMP_RE.is_match(default_str)
    }

    /// Tests whether the current database supports check constraints
    fn supports_check_constraints(&self) -> bool {
        self.circumstances.contains(Circumstances::CheckConstraints)
    }
}

async fn push_foreign_keys(
    schema_name: &str,
    table_ids: &IndexMap<String, TableId>,
    sql_schema: &mut SqlSchema,
    conn: &dyn Queryable,
) -> DescriberResult<()> {
    // We alias all the columns because MySQL column names are case-insensitive in queries, but
    // the information schema column names became upper-case in MySQL 8, causing the code
    // fetching the result values by column name below to fail.
    let sql = include_str!("kingbase_mysql/foreign_keys_query.sql");

    fn get_ids(
        row: &ResultRow,
        table_ids: &IndexMap<String, TableId>,
        sql_schema: &SqlSchema,
    ) -> Option<(TableId, TableColumnId, TableId, TableColumnId)> {
        let table_name = row.get_expect_string("table_name");
        let column_name = row.get_expect_string("column_name");
        let referenced_table_name = row.get_expect_string("referenced_table_name");
        let referenced_column_name = row.get_expect_string("referenced_column_name");

        let table_id = *table_ids.get(&table_name)?;
        let referenced_table_id = *table_ids.get(&referenced_table_name)?;
        let column_id = sql_schema.walk(table_id).column(&column_name)?.id;
        let referenced_column_id = sql_schema.walk(referenced_table_id).column(&referenced_column_name)?.id;

        Some((table_id, column_id, referenced_table_id, referenced_column_id))
    }

    let result_set = conn.query_raw(sql, &[schema_name.into()]).await?;
    let mut current_fk: Option<(TableId, String, ForeignKeyId)> = None;

    for row in result_set.into_iter() {
        trace!("Got description FK row {row:#?}");
        let table_name = row.get_expect_string("table_name");
        if !table_ids.contains_key(&table_name) {
            continue;
        }
        let referenced_schema = row.get_expect_string("referenced_table_schema");
        // Validate before resolving table names, even when the selected schema
        // contains a same-named target or contains no target at all.
        if referenced_schema != schema_name {
            return Err(DescriberErrorKind::CrossSchemaReference {
                from: format!("{schema_name}.{table_name}"),
                to: format!("{referenced_schema}.{}", row.get_expect_string("referenced_table_name")),
                constraint: row.get_expect_string("constraint_name"),
                missing_namespace: referenced_schema,
            }
            .into());
        }
        let (table_id, column_id, referenced_table_id, referenced_column_id) =
            if let Some(ids) = get_ids(&row, table_ids, sql_schema) {
                ids
            } else {
                continue;
            };
        let constraint_name = row.get_expect_string("constraint_name");
        let on_delete_action = match row.get_expect_string("delete_rule").to_lowercase().as_str() {
            "cascade" => ForeignKeyAction::Cascade,
            "set null" => ForeignKeyAction::SetNull,
            "set default" => ForeignKeyAction::SetDefault,
            "restrict" => ForeignKeyAction::Restrict,
            "no action" => ForeignKeyAction::NoAction,
            s => panic!("Unrecognized on delete action '{s}'"),
        };
        let on_update_action = match row.get_expect_string("update_rule").to_lowercase().as_str() {
            "cascade" => ForeignKeyAction::Cascade,
            "set null" => ForeignKeyAction::SetNull,
            "set default" => ForeignKeyAction::SetDefault,
            "restrict" => ForeignKeyAction::Restrict,
            "no action" => ForeignKeyAction::NoAction,
            s => panic!("Unrecognized on update action '{s}'"),
        };

        match &current_fk {
            Some((cur_table_id, cur_constraint_name, _))
                if *cur_table_id == table_id && *cur_constraint_name == constraint_name => {}
            None | Some(_) => {
                let fkid = sql_schema.push_foreign_key(
                    Some(constraint_name.clone()),
                    [table_id, referenced_table_id],
                    [on_delete_action, on_update_action],
                );

                current_fk = Some((table_id, constraint_name, fkid));
            }
        }

        if let Some((_, _, fkid)) = current_fk {
            sql_schema.push_foreign_key_column(fkid, [column_id, referenced_column_id]);
        }
    }

    Ok(())
}

fn push_enum_variants(full_data_type: &str, enum_id: EnumId, sql_schema: &mut SqlSchema) {
    let len = &full_data_type.len() - 1;
    // full_data_type for enum columns follows the pattern "enum('a','b')"
    let vals = &full_data_type[5..len];
    for variant in vals.split(',').map(unquote_string) {
        sql_schema.push_enum_variant(enum_id, variant.replace("''", "'"));
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EnumColumn, KingbaseMySqlType, SqlSchemaDescriber, fulltext_index_columns, index_expression_is_prefix,
        numeric_default_expressions_match, prefix_index_column, push_index_rows,
    };
    use crate::{ColumnArity, ColumnTypeFamily, DefaultValue, Precision, PrismaValue, SqlSchema};

    #[test]
    fn enum_columns_use_catalog_metadata_instead_of_displayed_type_names() {
        let metadata = EnumColumn {
            namespace: "other".into(),
            name: "Enum_status".into(),
            variants: vec!["draft".into(), "west,east".into(), "O'Reilly".into()],
        };
        for displayed_type in ["Enum_status", "other.Enum_status", "\"other\".\"Enum_status\""] {
            let mut schema = SqlSchema::default();
            let column = SqlSchemaDescriber::get_column_type(
                ("product", "status"),
                ("USER-DEFINED", displayed_type),
                Precision {
                    character_maximum_length: None,
                    numeric_precision: None,
                    numeric_scale: None,
                    time_precision: None,
                },
                ColumnArity::Nullable,
                &mut schema,
                Some(&metadata),
            );
            let ColumnTypeFamily::Enum(id) = column.family else {
                panic!("expected enum for {displayed_type}");
            };
            let described = schema.walk(id);
            assert_eq!(described.name(), "product_status");
            assert_eq!(
                described.values().collect::<Vec<_>>(),
                vec!["draft", "west,east", "O'Reilly"]
            );
        }
    }

    #[test]
    fn missing_enum_catalog_metadata_does_not_produce_an_empty_enum() {
        let column = SqlSchemaDescriber::get_column_type(
            ("product", "status"),
            ("USER-DEFINED", "Enum_unknown"),
            Precision {
                character_maximum_length: None,
                numeric_precision: None,
                numeric_scale: None,
                time_precision: None,
            },
            ColumnArity::Nullable,
            &mut SqlSchema::default(),
            None,
        );
        assert_eq!(column.family, ColumnTypeFamily::Unsupported("Enum_unknown".into()));
    }

    #[test]
    fn temporal_expressions_remain_sql_without_default_generated() {
        for expression in [
            "CURRENT_DATE",
            "CURRENT_TIME",
            "now()",
            "curdate()",
            "make_date(2020, 1, 2)",
            "((CURRENT_DATE)::bigint + 1)",
            "(now())::date",
            "CAST('2020-01-01' AS DATE)",
            "'2020-01-01'::date + 1",
        ] {
            let default = SqlSchemaDescriber::temporal_default(expression, Some(&KingbaseMySqlType::Date), Some(false));
            assert_eq!(default, DefaultValue::db_generated(expression), "{expression}");
        }
        for expression in ["CURRENT_TIMESTAMP", "current_timestamp()", "CURRENT_TIMESTAMP(3)"] {
            assert_eq!(
                SqlSchemaDescriber::temporal_default(
                    expression,
                    Some(&KingbaseMySqlType::DateTime(Some(3))),
                    Some(false)
                ),
                DefaultValue::now(),
                "{expression}"
            );
        }
    }

    #[test]
    fn temporal_literal_casts_are_removed_only_for_the_same_column_type() {
        for (expression, native, expected) in [
            ("'2020-01-01'::date", KingbaseMySqlType::Date, "'2020-01-01'"),
            ("'16:20:00'::time", KingbaseMySqlType::Time(Some(0)), "'16:20:00'"),
            (
                "'2020-01-01 16:20:00'::timestamp",
                KingbaseMySqlType::Timestamp(Some(3)),
                "'2020-01-01 16:20:00'",
            ),
            (
                "'2020-01-01 16:20:00'::datetime",
                KingbaseMySqlType::DateTime(Some(3)),
                "'2020-01-01 16:20:00'",
            ),
            ("'2020-01-01'", KingbaseMySqlType::Date, "'2020-01-01'"),
            (
                "'2020-01-01 16:20:00'::date",
                KingbaseMySqlType::DateTime(Some(3)),
                "'2020-01-01 16:20:00'::date",
            ),
            (
                "'16:20:00.123'::time(0)",
                KingbaseMySqlType::Time(Some(3)),
                "'16:20:00.123'::time(0)",
            ),
            (
                "'2020-01-01'::unknown",
                KingbaseMySqlType::Date,
                "'2020-01-01'::unknown",
            ),
        ] {
            assert_eq!(
                SqlSchemaDescriber::temporal_default(expression, Some(&native), Some(false)),
                DefaultValue::db_generated(expected),
                "{expression}"
            );
        }
    }

    #[test]
    fn temporal_casts_are_preserved_when_explicit_or_their_origin_is_unknown() {
        for (expression, native, literal) in [
            (
                "'16:20:00.123000'::time",
                KingbaseMySqlType::Time(Some(3)),
                "'16:20:00.123000'",
            ),
            (
                "'2020-01-01 16:20:00.123'::timestamp",
                KingbaseMySqlType::Timestamp(Some(3)),
                "'2020-01-01 16:20:00.123'",
            ),
        ] {
            assert_eq!(
                SqlSchemaDescriber::temporal_default(expression, Some(&native), Some(false)),
                DefaultValue::db_generated(literal),
            );
            for origin in [Some(true), None] {
                assert_eq!(
                    SqlSchemaDescriber::temporal_default(expression, Some(&native), origin),
                    DefaultValue::db_generated(expression),
                );
            }
        }
    }

    #[test]
    fn prefix_keys_require_prefix_nodes_and_preserve_quoted_names() {
        let tree =
            r#"({FUNCEXPR :args ({CONST :value "{PREFIXINDEX fake}"})} {PREFIXINDEX :expr {VAR :varattno 2} :len 8})"#;
        assert!(!index_expression_is_prefix(Some(tree), 0));
        assert!(index_expression_is_prefix(Some(tree), 1));
        assert!(!index_expression_is_prefix(Some(tree), 2));
        assert!(!index_expression_is_prefix(None, 0));
        for (definition, name, length) in [
            ("value(8)", "value", 8),
            (r#""x y"(4)"#, "x y", 4),
            (r#""q""q"(12)"#, "q\"q", 12),
            (r#""汉字"(3)"#, "汉字", 3),
            (r#""a(b"(2)"#, "a(b", 2),
        ] {
            assert_eq!(prefix_index_column(definition), Some((name.into(), length)));
        }
        for expression in [
            "lower(value)",
            "value(0)",
            "value(-1)",
            "value(4294967296)",
            "value(8) DESC",
            "value(8)::text",
            "schema.value(8)",
            r#""bad"name"(8)"#,
        ] {
            assert_eq!(prefix_index_column(expression), None, "{expression}");
        }
    }

    #[test]
    fn prefix_index_rows_restore_compound_keys_and_skip_arbitrary_expressions() {
        use crate::{Column, ColumnType, SQLSortOrder, kingbase_mysql::KingbaseMysqlSchemaExt};
        use indexmap::IndexMap;
        use quaint::{ast::Value, connector::ResultSet};

        let mut schema = SqlSchema::default();
        let namespace = schema.push_namespace("public".into());
        let table = schema.push_table("items".into(), namespace, None);
        for (name, family) in [
            ("id", ColumnTypeFamily::Int),
            ("value", ColumnTypeFamily::String),
            ("abs", ColumnTypeFamily::String),
            ("q\"q", ColumnTypeFamily::Binary),
        ] {
            schema.push_table_column(
                table,
                Column {
                    name: name.into(),
                    tpe: ColumnType::pure(family, ColumnArity::Required),
                    auto_increment: false,
                    description: None,
                },
            );
        }
        let columns = [
            "table_name",
            "index_name",
            "index_description",
            "column_name",
            "partial",
            "seq_in_index",
            "column_order",
            "non_unique",
            "index_type",
            "is_primary",
            "constraint_name",
            "index_expression",
            "index_expression_nodes",
            "index_column_definition",
        ];
        let mut rows = Vec::new();
        let prefix_nodes =
            "({PREFIXINDEX :expr {VAR :varattno 2} :len 8} {PREFIXINDEX :expr {VAR :varattno 4} :len 4})";
        let function_nodes = "({FUNCEXPR :args ({CONST :value 8})} {PREFIXINDEX :expr {VAR :varattno 2} :len 8})";
        // An ordinary key before and after an unsupported expression must not
        // survive as an incorrectly shortened compound index.
        for (index, unique, seq, name, definition, nodes) in [
            ("_prisma_idx_prefix", true, 1, None, "value(8)", prefix_nodes),
            ("_prisma_idx_prefix", true, 2, Some("id"), "id", prefix_nodes),
            ("_prisma_idx_prefix", true, 3, None, r#""q""q"(4)"#, prefix_nodes),
            ("expression", false, 1, Some("id"), "id", function_nodes),
            ("expression", false, 2, None, "abs(8)", function_nodes),
            ("expression", false, 3, None, "value(8)", function_nodes),
            ("following", false, 1, Some("id"), "id", ""),
            (
                "prefix_then_expression",
                false,
                1,
                None,
                "value(8)",
                "({PREFIXINDEX :expr {VAR} :len 8} {FUNCEXPR :args ({VAR})})",
            ),
            (
                "prefix_then_expression",
                false,
                2,
                None,
                "lower(value)",
                "({PREFIXINDEX :expr {VAR} :len 8} {FUNCEXPR :args ({VAR})})",
            ),
        ] {
            rows.push(vec![
                Value::text("items"),
                Value::text(index),
                Value::text("prisma:logical-index:logical_prefix"),
                name.map(Value::text).unwrap_or(Value::null_text()),
                Value::null_int32(),
                Value::int64(seq),
                Value::text(if seq == 3 { "D" } else { "A" }),
                Value::boolean(!unique),
                Value::text("btree"),
                Value::boolean(false),
                Value::null_text(),
                Value::null_text(),
                Value::text(nodes),
                Value::text(definition),
            ]);
        }
        // Catalog row order must not affect grouping or expression positions.
        rows.reverse();
        push_index_rows(
            &IndexMap::from([("items".into(), table)]),
            &mut schema,
            ResultSet::new(columns.into_iter().map(str::to_owned).collect(), vec![], rows),
        );
        let indexes = schema.walk(table).indexes().collect::<Vec<_>>();
        assert_eq!(indexes.len(), 2);
        let prefix = indexes.iter().find(|index| index.name() == "logical_prefix").unwrap();
        assert!(prefix.is_unique());
        assert_eq!(
            prefix
                .columns()
                .map(|column| (column.as_column().name(), column.length(), column.sort_order()))
                .collect::<Vec<_>>(),
            vec![
                ("value", Some(8), Some(SQLSortOrder::Asc)),
                ("id", None, Some(SQLSortOrder::Asc)),
                ("q\"q", Some(4), Some(SQLSortOrder::Desc))
            ]
        );
        let ext = schema.downcast_connector_data::<KingbaseMysqlSchemaExt>();
        assert_eq!(ext.physical_index_names.get(&prefix.id).unwrap(), "_prisma_idx_prefix");
        assert!(!ext.unique_constraint_names.contains_key(&prefix.id));
        assert_eq!(
            indexes
                .iter()
                .find(|index| index.name() == "following")
                .unwrap()
                .columns()
                .count(),
            1
        );
    }

    #[test]
    fn unsigned_integer_aliases_preserve_native_types_across_catalog_versions() {
        for (data_type, full_data_type, family, native_type) in [
            ("uint4", "uint4", ColumnTypeFamily::Int, KingbaseMySqlType::UnsignedInt),
            (
                "tinyint unsigned",
                "tinyint unsigned",
                ColumnTypeFamily::Int,
                KingbaseMySqlType::UnsignedTinyInt,
            ),
            (
                "smallint unsigned",
                "smallint unsigned",
                ColumnTypeFamily::Int,
                KingbaseMySqlType::UnsignedSmallInt,
            ),
            (
                "mediumint unsigned",
                "mediumint unsigned",
                ColumnTypeFamily::Int,
                KingbaseMySqlType::UnsignedMediumInt,
            ),
            (
                "int unsigned",
                "int unsigned",
                ColumnTypeFamily::Int,
                KingbaseMySqlType::UnsignedInt,
            ),
            (
                "integer unsigned",
                "integer unsigned",
                ColumnTypeFamily::Int,
                KingbaseMySqlType::UnsignedInt,
            ),
            (
                " INT UNSIGNED ",
                "INT UNSIGNED",
                ColumnTypeFamily::Int,
                KingbaseMySqlType::UnsignedInt,
            ),
            (
                "int",
                "int(10) unsigned",
                ColumnTypeFamily::Int,
                KingbaseMySqlType::UnsignedInt,
            ),
            (
                "int(10) unsigned",
                "int(10) unsigned",
                ColumnTypeFamily::Int,
                KingbaseMySqlType::UnsignedInt,
            ),
            (
                "uint8",
                "uint8",
                ColumnTypeFamily::BigInt,
                KingbaseMySqlType::UnsignedBigInt,
            ),
            (
                "bigint unsigned",
                "bigint unsigned",
                ColumnTypeFamily::BigInt,
                KingbaseMySqlType::UnsignedBigInt,
            ),
            (
                "BIGINT UNSIGNED",
                "BIGINT UNSIGNED",
                ColumnTypeFamily::BigInt,
                KingbaseMySqlType::UnsignedBigInt,
            ),
            (
                "bigint",
                "bigint(20) unsigned",
                ColumnTypeFamily::BigInt,
                KingbaseMySqlType::UnsignedBigInt,
            ),
        ] {
            for arity in [ColumnArity::Required, ColumnArity::Nullable] {
                let column_type = SqlSchemaDescriber::get_column_type(
                    ("integers", "value"),
                    (data_type, full_data_type),
                    Precision {
                        character_maximum_length: None,
                        numeric_precision: None,
                        numeric_scale: None,
                        time_precision: None,
                    },
                    arity,
                    &mut SqlSchema::default(),
                    None,
                );
                assert_eq!(column_type.family, family, "{data_type}");
                assert_eq!(column_type.arity, arity);
                assert_eq!(column_type.full_data_type, full_data_type);
                assert_eq!(
                    column_type
                        .native_type
                        .as_ref()
                        .unwrap()
                        .downcast_ref::<KingbaseMySqlType>(),
                    &native_type,
                    "{data_type}"
                );
            }
        }
    }

    #[test]
    fn signed_integers_keep_their_mapping() {
        for (data_type, family, native_type) in [
            ("int", ColumnTypeFamily::Int, Some(KingbaseMySqlType::Int)),
            ("integer", ColumnTypeFamily::Int, Some(KingbaseMySqlType::Int)),
            ("bigint", ColumnTypeFamily::BigInt, Some(KingbaseMySqlType::BigInt)),
            ("tinyint", ColumnTypeFamily::Int, Some(KingbaseMySqlType::TinyInt)),
            ("smallint", ColumnTypeFamily::Int, Some(KingbaseMySqlType::SmallInt)),
            ("mediumint", ColumnTypeFamily::Int, Some(KingbaseMySqlType::MediumInt)),
        ] {
            let column_type = SqlSchemaDescriber::get_column_type(
                ("integers", "value"),
                (data_type, data_type),
                Precision {
                    character_maximum_length: None,
                    numeric_precision: None,
                    numeric_scale: None,
                    time_precision: None,
                },
                ColumnArity::Nullable,
                &mut SqlSchema::default(),
                None,
            );
            assert_eq!(column_type.family, family, "{data_type}");
            assert_eq!(
                column_type
                    .native_type
                    .as_ref()
                    .map(|native| native.downcast_ref::<KingbaseMySqlType>()),
                native_type.as_ref(),
                "{data_type}"
            );
        }
    }

    #[test]
    fn numeric_expression_comparison_ignores_only_lossless_catalog_casts() {
        for (previous, next) in [
            ("(CAST('1.25' AS DECIMAL(10,1)))", "1.25::numeric(10,1)"),
            ("CAST(1.25 AS DECIMAL(10,1))", "(1.25::numeric(3,2))::numeric(10,1)"),
            (
                "CAST('-1.25' AS DECIMAL(10,1))",
                "('-1.25'::numeric(3,2))::numeric(10,1)",
            ),
            (
                "CAST(CAST('1.25' AS DECIMAL(10,1)) AS DECIMAL(65,30))",
                "((1.25::numeric(3,2))::numeric(10,1))::numeric(65,30)",
            ),
            ("1.25::numeric(10,1)", "(1.25::numeric(3,2))::numeric(10,1)"),
            ("1.23456::numeric(10,2)", "(1.23456::numeric(6,5))::numeric(10,2)"),
            ("'-1.25'::numeric(10,1)", "('-1.25'::numeric(3,2))::numeric(10,1)"),
            (
                "(1.25::numeric(10,1))::numeric(65,30)",
                "((1.25::numeric(3,2))::numeric(10,1))::numeric(65,30)",
            ),
            ("1.2500::numeric(10,1)", "((1.25::decimal(3,2))::decimal(10,1))"),
            ("1e-30::numeric(31,30)", "0.000000000000000000000000000001::numeric"),
            ("1.25::numeric(10,1)", "(1.25::numeric(3,2))::numeric(10,1)::numeric"),
        ] {
            assert!(numeric_default_expressions_match(previous, next), "{previous} / {next}");
            assert!(numeric_default_expressions_match(next, previous), "{next} / {previous}");
        }
    }

    #[test]
    fn numeric_expression_comparison_preserves_semantic_casts_and_rejects_unknown_sql() {
        for (previous, next) in [
            ("CAST('1.25' AS DECIMAL(10,1))", "1.25::numeric(10,2)"),
            ("CAST('1.35' AS DECIMAL(10,1))", "1.25::numeric(10,1)"),
            ("CAST('1.25' AS DECIMAL(10,1))", "CAST('1.25' AS DECIMAL(10,2))"),
            ("TRY_CAST('1.25' AS DECIMAL(10,1))", "1.25::numeric(10,1)"),
            ("CAST('1.25' AS DOUBLE)", "1.25::numeric"),
            ("1.25::numeric(10,1)", "1.25::numeric(10,2)"),
            ("1.25::numeric(10,1)", "1.35::numeric(10,1)"),
            ("1.25::numeric(10,1)", "1.3::numeric(10,1)"),
            ("(1.25::numeric(10,1))::numeric(10,2)", "1.25::numeric(10,2)"),
            ("1000::numeric(3,0)", "1000::numeric(4,0)"),
            ("(1.25::real)::numeric(10,1)", "1.25::numeric(10,1)"),
            ("'1.25'::text::numeric(10,1)", "1.25::numeric(10,1)"),
            ("1.25::public.numeric", "1.25::numeric"),
            ("1.25::numeric(10,-1)", "1.25::numeric(10,0)"),
            ("1.25::numeric(0,1)", "1.25::numeric(10,1)"),
            ("1.25::numeric(1001,1)", "1.25::numeric(10,1)"),
            ("round(1.25,1)", "ROUND((1.25),1)"),
            ("(1.25+1)::numeric(10,1)", "(1.25::numeric+1)::numeric(10,1)"),
            ("CAST('1.25' AS DECIMAL)", "1.25::numeric"),
            ("1.25::numeric(10,1) + 1", "1.25::numeric(10,1)"),
            ("1.25::numeric(10,1); SELECT 1", "1.25::numeric(10,1)"),
            ("'1.25::numeric(10,1)'", "1.25::numeric(10,1)"),
        ] {
            assert!(
                !numeric_default_expressions_match(previous, next),
                "{previous} / {next}"
            );
            assert!(
                !numeric_default_expressions_match(next, previous),
                "{next} / {previous}"
            );
        }
    }

    #[test]
    fn numeric_defaults_only_normalize_complete_literals_and_neutral_casts() {
        use std::str::FromStr;

        for native in [
            KingbaseMySqlType::Decimal(Some((10, 2))),
            KingbaseMySqlType::Float,
            KingbaseMySqlType::Double,
        ] {
            for (expression, expected) in [
                ("1.25", "1.25"),
                ("-1.25", "-1.25"),
                ("'1.25'", "1.25"),
                ("1.25::numeric", "1.25"),
                ("'1.25'::numeric::decimal", "1.25"),
                (" -1.25 :: sys_catalog.numeric ", "-1.25"),
                ("1.25::pg_catalog.numeric", "1.25"),
                ("1.25::numeric(3,2)", "1.25"),
                ("'-1.25'::numeric(3,2)", "-1.25"),
                ("1.2500::decimal(10,2)", "1.25"),
                ("1.25::numeric(3,2)::numeric(10,2)", "1.25"),
                ("1000::numeric(4)", "1000"),
                ("0::numeric(1,1)", "0"),
                ("0.001::numeric(3,3)", "0.001"),
                ("0.001::numeric(2,3)", "0.001"),
                ("1e-30::numeric", "1e-30"),
                ("1e-30::numeric(31,30)", "1e-30"),
                (
                    "123456789012345678901234567890.125::numeric",
                    "123456789012345678901234567890.125",
                ),
            ] {
                assert_eq!(
                    SqlSchemaDescriber::parse_float_default(expression, Some(&native)),
                    Some(PrismaValue::Float(bigdecimal::BigDecimal::from_str(expected).unwrap())),
                    "{expression}/{native:?}",
                );
            }
        }
    }

    #[test]
    fn numeric_defaults_preserve_rounding_and_other_expressions() {
        for native in [
            KingbaseMySqlType::Decimal(Some((10, 2))),
            KingbaseMySqlType::Float,
            KingbaseMySqlType::Double,
        ] {
            for expression in [
                "1.25::numeric(10,1)",
                "1000::numeric(3,0)",
                "1000::numeric(3)",
                "1.25::numeric(2,2)",
                "1.25::numeric(0,2)",
                "1.25::numeric(1001,2)",
                "1.25::numeric(10,-1)",
                "1.25::numeric(10,2,1)",
                "'1.25'::numeric(10,1)",
                "1.23456::numeric(10,2)",
                "1.25::numeric(10,1)::numeric",
                "1.25::numeric::decimal(10,1)",
                "(1.25::numeric(10,1))::numeric",
                "1.25::integer::numeric",
                "'123'::varchar(1)::numeric",
                "1.25::custom_numeric",
                "1.25::public.numeric",
                "1.25::numeric[]",
                "1.25::numeric + 2",
                "CAST('1.25' AS DECIMAL(10,1))",
                "round(1.25,1)",
                "'1.25::numeric'",
                "'1.25' || '0'",
                "'1.25",
                "1.25'",
                "'1.25::numeric",
                "1.25::",
            ] {
                assert_eq!(
                    SqlSchemaDescriber::parse_float_default(expression, Some(&native)),
                    None,
                    "{expression}/{native:?}",
                );
            }
        }
    }

    #[test]
    fn floating_defaults_only_normalize_casts_at_the_target_precision() {
        for (expression, native, normalized) in [
            ("'1.25'::real", KingbaseMySqlType::Float, true),
            ("'1.25'::double precision", KingbaseMySqlType::Double, true),
            ("1.25::real::numeric", KingbaseMySqlType::Float, true),
            ("1.25::double precision::numeric", KingbaseMySqlType::Double, true),
            ("1.25::real", KingbaseMySqlType::Double, false),
            ("1.25::double precision", KingbaseMySqlType::Float, false),
            ("1.25::real::double precision", KingbaseMySqlType::Double, false),
            ("99999999::real::numeric(8,0)", KingbaseMySqlType::Float, false),
            ("1.25::real", KingbaseMySqlType::Decimal(Some((10, 2))), false),
        ] {
            assert_eq!(
                SqlSchemaDescriber::parse_float_default(expression, Some(&native)).is_some(),
                normalized,
                "{expression}/{native:?}",
            );
        }
    }

    #[test]
    fn string_defaults_only_drop_neutral_character_casts() {
        for (input, literal) in [
            ("'abc'::varchar", "'abc'"),
            ("'abc'::text::character varying", "'abc'"),
            ("'O''Reilly'::varchar", "'O''Reilly'"),
            ("'ab::cd'::text", "'ab::cd'"),
            ("''::text", "''"),
            ("'字'::varchar", "'字'"),
        ] {
            assert_eq!(
                SqlSchemaDescriber::string_default_literal(input, Some(&KingbaseMySqlType::VarChar(30))),
                Some(literal),
                "{input}"
            );
        }
    }

    #[test]
    fn bpchar_defaults_only_normalize_for_character_targets() {
        let expression = "'abc '::bpchar";
        assert_eq!(
            SqlSchemaDescriber::string_default_literal(expression, Some(&KingbaseMySqlType::Char(5))),
            Some("'abc '")
        );
        for native in [KingbaseMySqlType::Text, KingbaseMySqlType::VarChar(30)] {
            assert_eq!(
                SqlSchemaDescriber::string_default_literal(expression, Some(&native)),
                None
            );
        }
    }

    #[test]
    fn string_defaults_preserve_semantic_and_unknown_casts() {
        for expression in [
            "'abcdef'::character(3)",
            "'abcdef'::character(3 char)",
            "'abcdef'::varchar(3)",
            "'abcdef'::text::varchar(3)",
            "'abcdef'::citext",
            "'abc '::bpchar",
            "'abcdef'::char",
            "'a'::text || 'b'::text",
            "'unfinished",
            "pending",
            "upper('abc'::text)",
            "NULLIF('a'::text, 'a'::text)",
            "E'line\\nnext'",
        ] {
            assert_eq!(
                SqlSchemaDescriber::string_default_literal(expression, Some(&KingbaseMySqlType::VarChar(30))),
                None,
                "{expression}"
            );
        }
    }

    #[test]
    fn ordinary_default_literals_are_decoded_only_once() {
        for (literal, expected) in [
            ("'a''''b'", "a''b"),
            ("'a''''''b'", "a'''b"),
            ("''''", "'"),
            ("''", ""),
            (r"'C:\\path'", r"C:\\path"),
            (r"'C:\''path'", r"C:\'path"),
            ("'_utf8mb4'", "_utf8mb4"),
        ] {
            assert_eq!(SqlSchemaDescriber::unquote_default_literal(literal), expected);
        }
    }

    #[test]
    fn enum_literal_defaults_accept_only_neutral_and_target_casts() {
        for expression in [
            "'A'",
            "'A'::Enum_123",
            "'A'::text::\"Enum_123\"",
            "'A'::public.\"Enum_123\"",
        ] {
            assert_eq!(
                SqlSchemaDescriber::enum_default_literal(expression, "public", "Enum_123"),
                Some("'A'")
            );
        }
        assert_eq!(
            SqlSchemaDescriber::enum_default_literal("'A'::\"n\"\"s::x\".\"Enum_123\"", "n\"s::x", "Enum_123"),
            Some("'A'")
        );
    }

    #[test]
    fn enum_default_expressions_and_semantic_casts_remain_sql() {
        for expression in [
            "'AB'::varchar(1)::Enum_123",
            "'A'::OtherEnum",
            "'A' || 'B'",
            "lower('A')::Enum_123",
            "E'A'::Enum_123",
            "'unfinished",
        ] {
            assert_eq!(
                SqlSchemaDescriber::enum_default_literal(expression, "public", "Enum_123"),
                None,
                "{expression}"
            );
        }
    }

    #[test]
    fn recognizes_the_renderer_fulltext_expression() {
        let expression = "to_tsvector('simple'::regconfig, textcat((COALESCE(title, ''::varchar))::text, textcat(' '::text, (COALESCE(content, ''::varchar))::text)))";

        assert_eq!(
            fulltext_index_columns(Some(expression)),
            Some(vec!["title".to_owned(), "content".to_owned()])
        );
    }

    #[test]
    fn leaves_other_tsvector_expressions_as_expression_indexes() {
        let expression = "to_tsvector('simple', COALESCE(title, '') || COALESCE(content, ''))";

        assert_eq!(fulltext_index_columns(Some(expression)), None);
    }
}
