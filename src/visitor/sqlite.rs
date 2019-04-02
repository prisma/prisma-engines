use crate::{ast::*, visitor::Visitor};

#[cfg(feature = "sqlite")]
use sqlite::{Bindable, Result as SqliteResult, Statement};

#[cfg(feature = "rusqlite")]
use rusqlite::{
    types::{Null, ToSql, ToSqlOutput},
    Error as RusqlError,
};

/// A visitor for generating queries for an SQLite database. Requires that
/// `rusqlite` feature flag is selected.
pub struct Sqlite {
    parameters: Vec<ParameterizedValue>,
}

impl Visitor for Sqlite {
    const C_PARAM: &'static str = "?";
    const C_BACKTICK: &'static str = "`";
    const C_WILDCARD: &'static str = "%";

    fn build<Q>(query: Q) -> (String, Vec<ParameterizedValue>)
    where
        Q: Into<Query>,
    {
        let mut sqlite = Sqlite { parameters: Vec::new() };

        (Sqlite::visit_query(&mut sqlite, query.into()), sqlite.parameters)
    }

    fn add_parameter(&mut self, value: ParameterizedValue) {
        self.parameters.push(value);
    }

    fn visit_limit(&mut self, limit: Option<usize>) -> String {
        if let Some(limit) = limit {
            format!("LIMIT {}", limit)
        } else {
            format!("LIMIT {}", -1)
        }
    }

    fn visit_function(&mut self, fun: Function) -> String {
        let mut result = match fun.typ_ {
            FunctionType::RowNumber(fun_rownum) => {
                if fun_rownum.over.is_empty() {
                    String::from("ROW_NUMBER() OVER()")
                } else {
                    format!("ROW_NUMBER() OVER({})", self.visit_partitioning(fun_rownum.over))
                }
            }
            FunctionType::Count(fun_count) => {
                if fun_count.exprs.is_empty() {
                    String::from("COUNT()")
                } else {
                    format!("COUNT({})", self.visit_columns(fun_count.exprs))
                }
            }
            FunctionType::Distinct(fun_distinct) => {
                if fun_distinct.exprs.is_empty() {
                    String::from("DISTINCT()")
                } else {
                    format!("DISTINCT({})", self.visit_columns(fun_distinct.exprs))
                }
            }
        };

        if let Some(alias) = fun.alias {
            result.push_str(" AS ");
            result.push_str(&Self::delimited_identifiers(vec![alias]));
        }

        result
    }

    fn visit_partitioning(&mut self, over: Over) -> String {
        let mut result = Vec::new();

        if !over.partitioning.is_empty() {
            let mut parts = Vec::new();

            for partition in over.partitioning {
                parts.push(self.visit_column(partition))
            }

            result.push(format!("PARTITION BY {}", parts.join(", ")));
        }

        if !over.ordering.is_empty() {
            result.push(format!("ORDER BY {}", self.visit_ordering(over.ordering)));
        }

        result.join(" ")
    }

    fn visit_offset(&mut self, offset: usize) -> String {
        format!("OFFSET {}", offset)
    }
}

#[cfg(feature = "sqlite")]
impl Bindable for ParameterizedValue {
    #[inline]
    fn bind(self, statement: &mut Statement, i: usize) -> SqliteResult<()> {
        use ParameterizedValue as Pv;
        match self {
            Pv::Null => statement.bind(i, ()),
            Pv::Integer(integer) => statement.bind(i, integer),
            Pv::Real(float) => statement.bind(i, float),
            Pv::Text(string) => statement.bind(i, string.as_str()),

            // Sqlite3 doesn't have booleans so we match to ints
            Pv::Boolean(true) => statement.bind(i, 1),
            Pv::Boolean(false) => statement.bind(i, 0),
        }
    }
}

#[cfg(feature = "rusqlite")]
impl ToSql for ParameterizedValue {
    fn to_sql(&self) -> Result<ToSqlOutput, RusqlError> {
        let value = match self {
            ParameterizedValue::Null => ToSqlOutput::from(Null),
            ParameterizedValue::Integer(integer) => ToSqlOutput::from(*integer),
            ParameterizedValue::Real(float) => ToSqlOutput::from(*float),
            ParameterizedValue::Text(string) => ToSqlOutput::from(string.clone()),
            ParameterizedValue::Boolean(boo) => ToSqlOutput::from(*boo),
        };

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use crate::visitor::*;

    fn expected_values<T>(sql: &'static str, params: Vec<T>) -> (String, Vec<ParameterizedValue>)
    where
        T: Into<ParameterizedValue>,
    {
        (String::from(sql), params.into_iter().map(|p| p.into()).collect())
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
    fn test_select_star_from() {
        let expected_sql = "SELECT `musti`.* FROM `musti` LIMIT -1";
        let query = Select::from_table("musti");
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(Vec::<ParameterizedValue>::new(), params);
    }

    #[test]
    fn test_select_order_by() {
        let expected_sql = "SELECT `musti`.* FROM `musti` ORDER BY `foo`, `baz` ASC, `bar` DESC LIMIT -1";
        let query = Select::from_table("musti")
            .order_by("foo")
            .order_by("baz".ascend())
            .order_by("bar".descend());
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(Vec::<ParameterizedValue>::new(), params);
    }

    #[test]
    fn test_select_fields_from() {
        let expected_sql = "SELECT `paw`, `nose` FROM `cat`.`musti` LIMIT -1";
        let query = Select::from_table(("cat", "musti")).column("paw").column("nose");
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(Vec::<ParameterizedValue>::new(), params);
    }

    #[test]
    fn test_select_where_equals() {
        let expected = expected_values(
            "SELECT `naukio`.* FROM `naukio` WHERE `word` = ? LIMIT -1",
            vec!["meow"],
        );

        let query = Select::from_table("naukio").so_that("word".equals("meow"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_select_where_like() {
        let expected = expected_values(
            "SELECT `naukio`.* FROM `naukio` WHERE `word` LIKE ? LIMIT -1",
            vec!["%meow%"],
        );

        let query = Select::from_table("naukio").so_that("word".like("meow"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_select_where_not_like() {
        let expected = expected_values(
            "SELECT `naukio`.* FROM `naukio` WHERE `word` NOT LIKE ? LIMIT -1",
            vec!["%meow%"],
        );

        let query = Select::from_table("naukio").so_that("word".not_like("meow"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_select_where_begins_with() {
        let expected = expected_values(
            "SELECT `naukio`.* FROM `naukio` WHERE `word` LIKE ? LIMIT -1",
            vec!["meow%"],
        );

        let query = Select::from_table("naukio").so_that("word".begins_with("meow"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_select_where_not_begins_with() {
        let expected = expected_values(
            "SELECT `naukio`.* FROM `naukio` WHERE `word` NOT LIKE ? LIMIT -1",
            vec!["meow%"],
        );

        let query = Select::from_table("naukio").so_that("word".not_begins_with("meow"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_select_where_ends_into() {
        let expected = expected_values(
            "SELECT `naukio`.* FROM `naukio` WHERE `word` LIKE ? LIMIT -1",
            vec!["%meow"],
        );

        let query = Select::from_table("naukio").so_that("word".ends_into("meow"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_select_where_not_ends_into() {
        let expected = expected_values(
            "SELECT `naukio`.* FROM `naukio` WHERE `word` NOT LIKE ? LIMIT -1",
            vec!["%meow"],
        );

        let query = Select::from_table("naukio").so_that("word".not_ends_into("meow"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_select_and() {
        let expected_sql = "SELECT `naukio`.* FROM `naukio` WHERE ((`word` = ? AND `age` < ?) AND `paw` = ?) LIMIT -1";

        let expected_params = vec![
            ParameterizedValue::Text(String::from("meow")),
            ParameterizedValue::Integer(10),
            ParameterizedValue::Text(String::from("warm")),
        ];

        let conditions = "word".equals("meow").and("age".less_than(10)).and("paw".equals("warm"));

        let query = Select::from_table("naukio").so_that(conditions);

        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(expected_params, params);
    }

    #[test]
    fn test_select_and_different_execution_order() {
        let expected_sql = "SELECT `naukio`.* FROM `naukio` WHERE (`word` = ? AND (`age` < ? AND `paw` = ?)) LIMIT -1";

        let expected_params = vec![
            ParameterizedValue::Text(String::from("meow")),
            ParameterizedValue::Integer(10),
            ParameterizedValue::Text(String::from("warm")),
        ];

        let conditions = "word".equals("meow").and("age".less_than(10).and("paw".equals("warm")));

        let query = Select::from_table("naukio").so_that(conditions);

        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(expected_params, params);
    }

    #[test]
    fn test_select_or() {
        let expected_sql = "SELECT `naukio`.* FROM `naukio` WHERE ((`word` = ? OR `age` < ?) AND `paw` = ?) LIMIT -1";

        let expected_params = vec![
            ParameterizedValue::Text(String::from("meow")),
            ParameterizedValue::Integer(10),
            ParameterizedValue::Text(String::from("warm")),
        ];

        let conditions = "word".equals("meow").or("age".less_than(10)).and("paw".equals("warm"));

        let query = Select::from_table("naukio").so_that(conditions);

        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(expected_params, params);
    }

    #[test]
    fn test_select_negation() {
        let expected_sql =
            "SELECT `naukio`.* FROM `naukio` WHERE (NOT ((`word` = ? OR `age` < ?) AND `paw` = ?)) LIMIT -1";

        let expected_params = vec![
            ParameterizedValue::Text(String::from("meow")),
            ParameterizedValue::Integer(10),
            ParameterizedValue::Text(String::from("warm")),
        ];

        let conditions = "word"
            .equals("meow")
            .or("age".less_than(10))
            .and("paw".equals("warm"))
            .not();

        let query = Select::from_table("naukio").so_that(conditions);

        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(expected_params, params);
    }

    #[test]
    fn test_with_raw_condition_tree() {
        let expected_sql =
            "SELECT `naukio`.* FROM `naukio` WHERE (NOT ((`word` = ? OR `age` < ?) AND `paw` = ?)) LIMIT -1";

        let expected_params = vec![
            ParameterizedValue::Text(String::from("meow")),
            ParameterizedValue::Integer(10),
            ParameterizedValue::Text(String::from("warm")),
        ];

        let conditions = ConditionTree::not(ConditionTree::and(
            ConditionTree::or("word".equals("meow"), "age".less_than(10)),
            "paw".equals("warm"),
        ));

        let query = Select::from_table("naukio").so_that(conditions);

        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(expected_params, params);
    }

    #[test]
    fn test_simple_inner_join() {
        let expected_sql =
            "SELECT `users`.* FROM `users` INNER JOIN `posts` ON `users`.`id` = `posts`.`user_id` LIMIT -1";

        let query = Select::from_table("users")
            .inner_join("posts".on(("users", "id").equals(Column::from(("posts", "user_id")))));
        let (sql, _) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_additional_condition_inner_join() {
        let expected_sql =
            "SELECT `users`.* FROM `users` INNER JOIN `posts` ON (`users`.`id` = `posts`.`user_id` AND `posts`.`published` = ?) LIMIT -1";

        let query = Select::from_table("users").inner_join(
            "posts".on(("users", "id")
                .equals(Column::from(("posts", "user_id")))
                .and(("posts", "published").equals(true))),
        );

        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![ParameterizedValue::Boolean(true),], params);
    }

    #[test]
    fn test_simple_left_join() {
        let expected_sql =
            "SELECT `users`.* FROM `users` LEFT OUTER JOIN `posts` ON `users`.`id` = `posts`.`user_id` LIMIT -1";

        let query = Select::from_table("users")
            .left_outer_join("posts".on(("users", "id").equals(Column::from(("posts", "user_id")))));
        let (sql, _) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_additional_condition_left_join() {
        let expected_sql =
            "SELECT `users`.* FROM `users` LEFT OUTER JOIN `posts` ON (`users`.`id` = `posts`.`user_id` AND `posts`.`published` = ?) LIMIT -1";

        let query = Select::from_table("users").left_outer_join(
            "posts".on(("users", "id")
                .equals(Column::from(("posts", "user_id")))
                .and(("posts", "published").equals(true))),
        );

        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![ParameterizedValue::Boolean(true),], params);
    }

    #[test]
    fn test_column_aliasing() {
        let expected_sql = "SELECT `bar` AS `foo` FROM `meow` LIMIT -1";
        let query = Select::from_table("meow").column(Column::new("bar").alias("foo"));
        let (sql, _) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
    }

    /// Creates a simple sqlite database with a user table and a nice user
    #[cfg(feature = "sqlite")]
    fn sqlite_harness() -> ::sqlite::Connection {
        let conn = ::sqlite::open(":memory:").unwrap();

        conn.execute(
            "
            CREATE TABLE users (id, name TEXT, age REAL, nice INTEGER);
            INSERT INTO users (id, name, age, nice) VALUES (1, 'Alice', 42.69, 1);
            ",
        )
        .unwrap();

        conn
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn bind_test_1() {
        let conn = sqlite_harness();

        let conditions = "name"
            .equals("Alice")
            .and("age".less_than(100.0))
            .and("nice".equals(true));
        let query = Select::from_table("users").so_that(conditions);
        let (sql_str, params) = Sqlite::build(query);

        let mut s = conn.prepare(sql_str.clone()).unwrap();
        for i in 1..params.len() + 1 {
            s.bind::<ParameterizedValue>(i, params[i - 1].clone().into()).unwrap();
        }

        s.next().unwrap();

        assert_eq!("Alice", s.read::<String>(1).unwrap());
        assert_eq!(42.69, s.read::<f64>(2).unwrap());
        assert_eq!(1, s.read::<i64>(3).unwrap());
    }

    #[cfg(feature = "rusqlite")]
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

        let (sql, params) = dbg!(Sqlite::build(insert));

        conn.execute(&sql, params.as_slice()).unwrap();
        conn
    }

    #[test]
    #[cfg(feature = "rusqlite")]
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
            .query_map(&params, |row| Person {
                name: row.get(1),
                age: row.get(2),
                nice: row.get(3),
            })
            .unwrap();

        let person: Person = person_iter.nth(0).unwrap().unwrap();

        assert_eq!("Alice", person.name);
        assert_eq!(42.69, person.age);
        assert_eq!(1, person.nice);
    }
}
