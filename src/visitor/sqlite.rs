use crate::{ast::*, visitor::Visitor};

use std::fmt::{self, Write};

/// A visitor to generate queries for the SQLite database.
///
/// The returned parameter values implement the `ToSql` trait from rusqlite and
/// can be used directly with the database.
pub struct Sqlite<'a> {
    query: String,
    parameters: Vec<Value<'a>>,
}

impl<'a> Visitor<'a> for Sqlite<'a> {
    const C_BACKTICK: &'static str = "`";
    const C_WILDCARD: &'static str = "%";

    fn build<Q>(query: Q) -> (String, Vec<Value<'a>>)
    where
        Q: Into<Query<'a>>,
    {
        let mut sqlite = Sqlite {
            query: String::with_capacity(4096),
            parameters: Vec::with_capacity(128),
        };

        Sqlite::visit_query(&mut sqlite, query.into());

        (sqlite.query, sqlite.parameters)
    }

    fn write<D: fmt::Display>(&mut self, s: D) -> fmt::Result {
        write!(&mut self.query, "{}", s)
    }

    fn visit_insert(&mut self, insert: Insert<'a>) -> fmt::Result {
        match insert.on_conflict {
            Some(OnConflict::DoNothing) => self.write("INSERT OR IGNORE")?,
            None => self.write("INSERT")?,
        };

        self.write(" INTO ")?;
        self.visit_table(insert.table, true)?;

        if insert.values.is_empty() {
            self.write(" DEFAULT VALUES")?;
        } else {
            let columns = insert.columns.len();

            self.write(" (")?;
            for (i, c) in insert.columns.into_iter().enumerate() {
                self.visit_column(c)?;

                if i < (columns - 1) {
                    self.write(", ")?;
                }
            }
            self.write(")")?;

            self.write(" VALUES ")?;
            let values = insert.values.len();

            for (i, row) in insert.values.into_iter().enumerate() {
                self.visit_row(row)?;

                if i < (values - 1) {
                    self.write(", ")?;
                }
            }
        }

        Ok(())
    }

    fn parameter_substitution(&mut self) -> fmt::Result {
        self.write("?")
    }

    fn add_parameter(&mut self, value: Value<'a>) {
        self.parameters.push(value);
    }

    fn visit_limit_and_offset(&mut self, limit: Option<Value<'a>>, offset: Option<Value<'a>>) -> fmt::Result {
        match (limit, offset) {
            (Some(limit), Some(offset)) => {
                self.write(" LIMIT ")?;
                self.visit_parameterized(limit)?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(offset)
            }
            (None, Some(offset)) => {
                self.write(" LIMIT ")?;
                self.visit_parameterized(Value::from(-1))?;

                self.write(" OFFSET ")?;
                self.visit_parameterized(offset)
            }
            (Some(limit), None) => {
                self.write(" LIMIT ")?;
                self.visit_parameterized(limit)
            }
            (None, None) => Ok(()),
        }
    }

    fn visit_aggregate_to_string(&mut self, value: Expression<'a>) -> fmt::Result {
        self.write("GROUP_CONCAT")?;
        self.surround_with("(", ")", |ref mut s| s.visit_expression(value))
    }

    fn visit_values(&mut self, values: Values<'a>) -> fmt::Result {
        self.surround_with("(VALUES ", ")", |ref mut s| {
            let len = values.len();
            for (i, row) in values.into_iter().enumerate() {
                s.visit_row(row)?;

                if i < (len - 1) {
                    s.write(",")?;
                }
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{val, visitor::*};

    fn expected_values<'a, T>(sql: &'static str, params: Vec<T>) -> (String, Vec<Value<'a>>)
    where
        T: Into<Value<'a>>,
    {
        (String::from(sql), params.into_iter().map(|p| p.into()).collect())
    }

    fn default_params<'a>(mut additional: Vec<Value<'a>>) -> Vec<Value<'a>> {
        let mut result = Vec::new();

        for param in additional.drain(0..) {
            result.push(param)
        }

        result
    }

    #[test]
    fn test_select_1() {
        let expected = expected_values("SELECT ?", vec![1]);

        let query = Select::default().value(1);
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_aliased_value() {
        let expected = expected_values("SELECT ? AS `test`", vec![1]);

        let query = Select::default().value(val!(1).alias("test"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_aliased_null() {
        let expected_sql = "SELECT ? AS `test`";
        let query = Select::default().value(val!(Value::Null).alias("test"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::Null], params);
    }

    #[test]
    fn test_select_star_from() {
        let expected_sql = "SELECT `musti`.* FROM `musti`";
        let query = Select::from_table("musti");
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[test]
    fn test_select_from_values() {
        use crate::values;

        let expected_sql = "SELECT `vals`.* FROM (VALUES (?,?),(?,?)) AS `vals`";
        let values = Table::from(values!((1, 2), (3, 4))).alias("vals");
        let query = Select::from_table(values);
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(
            vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3),
                Value::Integer(4),
            ],
            params
        );
    }

    #[test]
    fn test_in_values() {
        use crate::{col, values};

        let expected_sql = "SELECT `test`.* FROM `test` WHERE (`id1`,`id2`) IN (VALUES (?,?),(?,?))";
        let query = Select::from_table("test")
            .so_that(Row::from((col!("id1"), col!("id2"))).in_selection(values!((1, 2), (3, 4))));

        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(
            vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3),
                Value::Integer(4),
            ],
            params
        );
    }

    #[test]
    fn test_in_values_singular() {
        let mut cols = Row::new();
        cols.push(Column::from("id1"));

        let mut vals = Values::new();

        {
            let mut row1 = Row::new();
            row1.push(1);

            let mut row2 = Row::new();
            row2.push(2);

            vals.push(row1);
            vals.push(row2);
        }

        let query = Select::from_table("test").so_that(cols.in_selection(vals));
        let (sql, params) = Sqlite::build(query);
        let expected_sql = "SELECT `test`.* FROM `test` WHERE `id1` IN (?,?)";

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![Value::Integer(1), Value::Integer(2),], params)
    }

    #[test]
    fn test_select_order_by() {
        let expected_sql = "SELECT `musti`.* FROM `musti` ORDER BY `foo`, `baz` ASC, `bar` DESC";
        let query = Select::from_table("musti")
            .order_by("foo")
            .order_by("baz".ascend())
            .order_by("bar".descend());
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[test]
    fn test_select_fields_from() {
        let expected_sql = "SELECT `paw`, `nose` FROM `cat`.`musti`";
        let query = Select::from_table(("cat", "musti")).column("paw").column("nose");
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![]), params);
    }

    #[test]
    fn test_select_where_equals() {
        let expected = expected_values("SELECT `naukio`.* FROM `naukio` WHERE `word` = ?", vec!["meow"]);

        let query = Select::from_table("naukio").so_that("word".equals("meow"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_like() {
        let expected = expected_values("SELECT `naukio`.* FROM `naukio` WHERE `word` LIKE ?", vec!["%meow%"]);

        let query = Select::from_table("naukio").so_that("word".like("meow"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_not_like() {
        let expected = expected_values(
            "SELECT `naukio`.* FROM `naukio` WHERE `word` NOT LIKE ?",
            vec!["%meow%"],
        );

        let query = Select::from_table("naukio").so_that("word".not_like("meow"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_begins_with() {
        let expected = expected_values("SELECT `naukio`.* FROM `naukio` WHERE `word` LIKE ?", vec!["meow%"]);

        let query = Select::from_table("naukio").so_that("word".begins_with("meow"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_not_begins_with() {
        let expected = expected_values("SELECT `naukio`.* FROM `naukio` WHERE `word` NOT LIKE ?", vec!["meow%"]);

        let query = Select::from_table("naukio").so_that("word".not_begins_with("meow"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_ends_into() {
        let expected = expected_values("SELECT `naukio`.* FROM `naukio` WHERE `word` LIKE ?", vec!["%meow"]);

        let query = Select::from_table("naukio").so_that("word".ends_into("meow"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_where_not_ends_into() {
        let expected = expected_values("SELECT `naukio`.* FROM `naukio` WHERE `word` NOT LIKE ?", vec!["%meow"]);

        let query = Select::from_table("naukio").so_that("word".not_ends_into("meow"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(default_params(expected.1), params);
    }

    #[test]
    fn test_select_and() {
        let expected_sql = "SELECT `naukio`.* FROM `naukio` WHERE (`word` = ? AND `age` < ? AND `paw` = ?)";

        let expected_params = vec![
            Value::Text(Cow::from("meow")),
            Value::Integer(10),
            Value::Text(Cow::from("warm")),
        ];

        let conditions = "word".equals("meow").and("age".less_than(10)).and("paw".equals("warm"));

        let query = Select::from_table("naukio").so_that(conditions);

        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_select_and_different_execution_order() {
        let expected_sql = "SELECT `naukio`.* FROM `naukio` WHERE (`word` = ? AND (`age` < ? AND `paw` = ?))";

        let expected_params = vec![
            Value::Text(Cow::from("meow")),
            Value::Integer(10),
            Value::Text(Cow::from("warm")),
        ];

        let conditions = "word".equals("meow").and("age".less_than(10).and("paw".equals("warm")));

        let query = Select::from_table("naukio").so_that(conditions);

        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_select_or() {
        let expected_sql = "SELECT `naukio`.* FROM `naukio` WHERE ((`word` = ? OR `age` < ?) AND `paw` = ?)";

        let expected_params = vec![
            Value::Text(Cow::from("meow")),
            Value::Integer(10),
            Value::Text(Cow::from("warm")),
        ];

        let conditions = "word".equals("meow").or("age".less_than(10)).and("paw".equals("warm"));

        let query = Select::from_table("naukio").so_that(conditions);

        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_select_negation() {
        let expected_sql = "SELECT `naukio`.* FROM `naukio` WHERE (NOT ((`word` = ? OR `age` < ?) AND `paw` = ?))";

        let expected_params = vec![
            Value::Text(Cow::from("meow")),
            Value::Integer(10),
            Value::Text(Cow::from("warm")),
        ];

        let conditions = "word"
            .equals("meow")
            .or("age".less_than(10))
            .and("paw".equals("warm"))
            .not();

        let query = Select::from_table("naukio").so_that(conditions);

        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_with_raw_condition_tree() {
        let expected_sql = "SELECT `naukio`.* FROM `naukio` WHERE (NOT ((`word` = ? OR `age` < ?) AND `paw` = ?))";

        let expected_params = vec![
            Value::Text(Cow::from("meow")),
            Value::Integer(10),
            Value::Text(Cow::from("warm")),
        ];

        let conditions = ConditionTree::not("word".equals("meow").or("age".less_than(10)).and("paw".equals("warm")));
        let query = Select::from_table("naukio").so_that(conditions);

        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(expected_params), params);
    }

    #[test]
    fn test_simple_inner_join() {
        let expected_sql = "SELECT `users`.* FROM `users` INNER JOIN `posts` ON `users`.`id` = `posts`.`user_id`";

        let query = Select::from_table("users")
            .inner_join("posts".on(("users", "id").equals(Column::from(("posts", "user_id")))));
        let (sql, _) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_additional_condition_inner_join() {
        let expected_sql =
            "SELECT `users`.* FROM `users` INNER JOIN `posts` ON (`users`.`id` = `posts`.`user_id` AND `posts`.`published` = ?)";

        let query = Select::from_table("users").inner_join(
            "posts".on(("users", "id")
                .equals(Column::from(("posts", "user_id")))
                .and(("posts", "published").equals(true))),
        );

        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![Value::Boolean(true),]), params);
    }

    #[test]
    fn test_simple_left_join() {
        let expected_sql = "SELECT `users`.* FROM `users` LEFT JOIN `posts` ON `users`.`id` = `posts`.`user_id`";

        let query = Select::from_table("users")
            .left_join("posts".on(("users", "id").equals(Column::from(("posts", "user_id")))));
        let (sql, _) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_additional_condition_left_join() {
        let expected_sql =
            "SELECT `users`.* FROM `users` LEFT JOIN `posts` ON (`users`.`id` = `posts`.`user_id` AND `posts`.`published` = ?)";

        let query = Select::from_table("users").left_join(
            "posts".on(("users", "id")
                .equals(Column::from(("posts", "user_id")))
                .and(("posts", "published").equals(true))),
        );

        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(default_params(vec![Value::Boolean(true),]), params);
    }

    #[test]
    fn test_column_aliasing() {
        let expected_sql = "SELECT `bar` AS `foo` FROM `meow`";
        let query = Select::from_table("meow").column(Column::new("bar").alias("foo"));
        let (sql, _) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
    }

    #[cfg(feature = "sqlite")]
    fn sqlite_harness() -> ::rusqlite::Connection {
        let conn = ::rusqlite::Connection::open_in_memory().unwrap();

        conn.execute(
            "CREATE TABLE users (id, name TEXT, age REAL, nice INTEGER)",
            ::rusqlite::NO_PARAMS,
        )
        .unwrap();

        let insert = Insert::single_into("users")
            .value("id", 1)
            .value("name", "Alice")
            .value("age", 42.69)
            .value("nice", true);

        let (sql, params) = Sqlite::build(insert);

        conn.execute(&sql, params.as_slice()).unwrap();
        conn
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn bind_test_1() {
        let conn = sqlite_harness();

        let conditions = "name".equals("Alice").and("age".less_than(100.0)).and("nice".equals(1));
        let query = Select::from_table("users").so_that(conditions);
        let (sql_str, params) = Sqlite::build(query);

        #[derive(Debug)]
        struct Person {
            name: String,
            age: f64,
            nice: i32,
        }

        let mut stmt = conn.prepare(&sql_str).unwrap();
        let mut person_iter = stmt
            .query_map(&params, |row| {
                Ok(Person {
                    name: row.get(1).unwrap(),
                    age: row.get(2).unwrap(),
                    nice: row.get(3).unwrap(),
                })
            })
            .unwrap();

        let person: Person = person_iter.nth(0).unwrap().unwrap();

        assert_eq!("Alice", person.name);
        assert_eq!(42.69, person.age);
        assert_eq!(1, person.nice);
    }
}
