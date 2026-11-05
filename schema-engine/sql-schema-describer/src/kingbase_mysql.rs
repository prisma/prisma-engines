//! Kingbase MySQL schema description.

use crate::{getters::Getter, parsers::Parser, *};
use indexmap::IndexMap;
use psl::{builtin_connectors::KingbaseMySqlType, datamodel_connector::NativeTypeInstance};
use quaint::prelude::{Queryable, ResultRow};
use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};
use tracing::trace;

/// Matches a default value in the schema, wrapped single quotes.
///
/// Example:
///
/// ```ignore
/// 'this is a test'
/// ```
static DEFAULT_QUOTES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"'(.*)'").unwrap());

fn is_mariadb(version: &str) -> bool {
    version.contains("MariaDB")
}

enum Flavour {
    Mysql,
    MariaDb,
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

pub const UNSIGNED_AUTO_GUARD_BODY: &str = "DECLARE v bigint; BEGIN v := jsonb_extract_path_text(to_jsonb(NEW),TG_ARGV[0])::bigint; IF v < 0 OR v > TG_ARGV[1]::bigint THEN RAISE EXCEPTION 'Value out of range for small unsigned integer' USING ERRCODE='23514'; END IF; RETURN NEW; END;";

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

        self.get_constraints(&table_names, &mut sql_schema).await?;

        let legacy_unsigned = Self::get_all_columns(&table_names, self.conn, schema, &mut sql_schema, &flavour).await?;
        push_foreign_keys(schema, &table_names, &mut sql_schema, self.conn, &legacy_unsigned).await?;
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
        let length = row.get_u32("partial");

        let sort_order = row.get_string("column_order").map(|v| match v.as_ref() {
            "A" => SQLSortOrder::Asc,
            "D" => SQLSortOrder::Desc,
            misc => panic!("Unexpected sort order `{misc}`, collation should be A, D or Null"),
        });

        let seq_in_index = row.get_expect_i64("seq_in_index"); // starts at 1

        if seq_in_index == 1 {
            current_index_id = None;
            skip_current_index = false;
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

        let column_name = if let Some(name) = row.get_string("column_name") {
            name
        } else {
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

    Ok(())
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
    ) -> DescriberResult<HashSet<TableColumnId>> {
        let user_defined_enum_variants = Self::get_user_defined_enum_variants(conn, schema_name).await?;
        let domain_columns = conn
            .query_raw(
                include_str!("kingbase_mysql/unsigned_domains_query.sql"),
                &[schema_name.into()],
            )
            .await?;
        let mut managed_unsigned: HashMap<_, _> = domain_columns
            .into_iter()
            .filter_map(|row| {
                let domain_name = row.get_string("domain_name")?;
                let maximum = match domain_name.as_str() {
                    "__prisma_uint8" => 255,
                    "__prisma_uint16" => 65535,
                    "__prisma_uint24" => 16777215,
                    _ => return None,
                };
                let domain_schema = row.get_expect_string("domain_schema");
                let type_name = format!(
                    "\"{}\".\"{}\"",
                    domain_schema.replace('"', "\"\""),
                    domain_name.replace('"', "\"\"")
                );
                let expression = row
                    .get_string("range_expression")?
                    .replace("OPERATOR(sys.<=)", "<=")
                    .replace(' ', "");
                (expression == format!("(VALUE<={maximum})")).then(|| {
                    (
                        (
                            row.get_expect_string("table_name"),
                            row.get_expect_string("column_name"),
                        ),
                        type_name,
                    )
                })
            })
            .collect();

        let auto_columns = conn
            .query_raw(
                include_str!("kingbase_mysql/unsigned_auto_increment_query.sql"),
                &[schema_name.into()],
            )
            .await?;
        for row in auto_columns {
            if row.get_expect_string("body") != UNSIGNED_AUTO_GUARD_BODY {
                continue;
            }
            let Some(arguments) = row.get("trigger_arguments").and_then(|value| value.as_bytes()) else {
                continue;
            };
            let arguments: Vec<_> = arguments.split(|byte| *byte == 0).collect();
            if arguments.len() != 3 || !arguments[2].is_empty() {
                continue;
            }
            let name = row.get_expect_string("column_name");
            if arguments[0] != name.as_bytes() {
                continue;
            }
            if !matches!(arguments[1], b"255" | b"65535" | b"16777215") {
                continue;
            }
            managed_unsigned.insert((row.get_expect_string("table_name"), name), "INTEGER".to_owned());
        }

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

            let mut tpe = Self::get_column_type(
                (&table_name, &name),
                (&data_type, &full_data_type),
                precision,
                arity,
                sql_schema,
                &user_defined_enum_variants,
            );
            if let Some(type_name) = managed_unsigned.get(&(table_name.clone(), name.clone())) {
                // Legacy emulation objects remain in the database. Expose them
                // through the framework's Unsupported/warning path, never as a
                // native type now rejected by PSL or as an unrestricted integer.
                tpe.family = ColumnTypeFamily::Unsupported(type_name.clone());
                tpe.full_data_type.clone_from(type_name);
                tpe.native_type = None;
            }
            let extra = col.get_string("extra").unwrap_or_default().to_lowercase();
            let auto_increment = matches!(extra.as_str(), "auto_increment")
                && !managed_unsigned.contains_key(&(table_name.clone(), name.clone()));

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
                            ColumnTypeFamily::Float => match Self::parse_float(
                                default_string
                                    .split_once("::")
                                    .map_or(&default_string, |(value, _)| value),
                            ) {
                                Some(float_value) => DefaultValue::value(float_value),
                                None if default_expression => Self::dbgenerated_expression(&default_string),
                                None => DefaultValue::db_generated(default_string),
                            },
                            ColumnTypeFamily::Decimal => match Self::parse_float(
                                default_string
                                    .split_once("::")
                                    .map_or(&default_string, |(value, _)| value),
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
                            ColumnTypeFamily::DateTime => {
                                // Kingbase exposes literal temporal defaults with a PostgreSQL
                                // type cast (for example `'2020-01-01'::date`). The cast is
                                // catalog decoration; retain the literal expected by the
                                // Prisma schema so a second schema push is idempotent.
                                let default_string = default_string
                                    .split_once("::")
                                    .filter(|(_, cast)| matches!(*cast, "date" | "time" | "timestamp"))
                                    .map_or(default_string.as_str(), |(value, _)| value);

                                match Self::default_is_current_timestamp(default_string) {
                                    true => DefaultValue::now(),
                                    _ if default_expression => Self::dbgenerated_expression(default_string),
                                    _ if DEFAULT_QUOTES.is_match(default_string) => {
                                        DefaultValue::db_generated(default_string.to_owned())
                                    }
                                    _ => DefaultValue::db_generated(format!("'{default_string}'")),
                                }
                            }
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
                                } else if let Some(literal) =
                                    Self::enum_default_literal(&default_string, schema_name, &full_data_type)
                                {
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

        Ok(managed_unsigned
            .keys()
            .filter_map(|(table, column)| {
                let table_id = *table_ids.get(table)?;
                Some(sql_schema.walk(table_id).column(column)?.id)
            })
            .collect())
    }

    async fn get_user_defined_enum_variants(
        conn: &dyn Queryable,
        schema_name: &str,
    ) -> DescriberResult<HashMap<String, Vec<String>>> {
        // Kingbase exposes MySQL ENUM columns through user-defined
        // types. The information_schema COLUMNS view only contains the generated type name,
        // so fetch the labels from sys_enum separately.
        let sql = include_str!("kingbase_mysql/enums_query.sql");

        let rows = conn.query_raw(sql, &[schema_name.into()]).await?;
        let mut variants = HashMap::new();

        for row in rows {
            let type_name = row.get_expect_string("type_name");
            let enum_label = row.get_expect_string("enum_label");
            variants.entry(type_name).or_insert_with(Vec::new).push(enum_label);
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
        user_defined_enum_variants: &HashMap<String, Vec<String>>,
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
            "uint4" => (ColumnTypeFamily::Int, Some(KingbaseMySqlType::UnsignedInt)),
            "uint8" => (ColumnTypeFamily::BigInt, Some(KingbaseMySqlType::UnsignedBigInt)),
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
            "user-defined"
                if full_data_type
                    .get(..5)
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case("enum_")) =>
            {
                let enum_name = format!("{table}_{column_name}");
                let enum_id = sql_schema.push_enum(Default::default(), enum_name, None);

                let variants = user_defined_enum_variants.get(full_data_type).or_else(|| {
                    user_defined_enum_variants
                        .iter()
                        .find(|(type_name, _)| type_name.eq_ignore_ascii_case(full_data_type))
                        .map(|(_, variants)| variants)
                });

                if let Some(variants) = variants {
                    for variant in variants {
                        sql_schema.push_enum_variant(enum_id, variant.clone());
                    }
                }

                (ColumnTypeFamily::Enum(enum_id), None)
            }
            "user-defined" if full_data_type.starts_with("Set_") => (ColumnTypeFamily::String, None),
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
        table_names: &IndexMap<String, TableId>,
        sql_schema: &mut SqlSchema,
    ) -> DescriberResult<()> {
        // Only MySQL 8.0.16 and above supports check constraints and has the CHECK_CONSTRAINTS table we can query.
        if !self.supports_check_constraints() {
            return Ok(());
        }

        // Note: in MySQL, columns must be re-aliased, otherwise their casing would be inconsistent (and uppercased).
        let sql = include_str!("mysql/constraints_query.sql");

        let rows = self.conn.query_raw(sql, &[]).await?;

        for row in rows {
            let table_name = row.get_expect_string("table_name");
            let constraint_name = row.get_expect_string("constraint_name");
            let constraint_type = row.get_expect_string("constraint_type");
            let constraint_definition = row.get_string("constraint_definition");

            let table_id = match table_names.get(&table_name) {
                Some(id) => *id,
                None => continue,
            };

            if constraint_type.as_str() == "check"
                && !constraint_definition
                    .as_deref()
                    .is_some_and(|definition| definition.to_ascii_lowercase().contains("is not null"))
            {
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
    legacy_unsigned: &HashSet<TableColumnId>,
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

    let result_set = conn.query_raw(sql, &[schema_name.into(), schema_name.into()]).await?;
    let mut current_fk: Option<(TableId, String, ForeignKeyId)> = None;

    for row in result_set.into_iter() {
        trace!("Got description FK row {row:#?}");
        let (table_id, column_id, referenced_table_id, referenced_column_id) =
            if let Some(ids) = get_ids(&row, table_ids, sql_schema) {
                ids
            } else {
                continue;
            };
        let constraint_name = row.get_expect_string("constraint_name");
        if (legacy_unsigned.contains(&column_id) || legacy_unsigned.contains(&referenced_column_id))
            && sql_schema.walk(column_id).column_type_family()
                != sql_schema.walk(referenced_column_id).column_type_family()
        {
            // Unsupported relation fields must have the same type in PSL.
            // Keep the actual physical types and constraints intact rather than
            // hiding a foreign key or emitting an invalid datamodel.
            return Err(quaint::error::Error::builder(quaint::error::ErrorKind::ConversionError(format!(
                "Legacy small unsigned emulation in foreign key `{constraint_name}` uses incompatible physical types. Migrate the legacy columns and foreign key explicitly before introspection or schema migration; Prisma does not emulate these types."
            ).into())).build().into());
        }
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
    use super::{KingbaseMySqlType, SqlSchemaDescriber, fulltext_index_columns};

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
