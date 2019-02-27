use crate::{ast::*, visitor::Visitor};

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
        let mut sqlite = Sqlite {
            parameters: Vec::new(),
        };

        (
            Sqlite::visit_query(&mut sqlite, query.into()),
            sqlite.parameters,
        )
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

    fn visit_offset(&mut self, offset: usize) -> String {
        format!("OFFSET {}", offset)
    }
}

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
        (
            String::from(sql),
            params.into_iter().map(|p| p.into()).collect(),
        )
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
        let expected_sql = "SELECT * FROM `musti` LIMIT -1";
        let query = Select::from("musti");
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(Vec::<ParameterizedValue>::new(), params);
    }

    #[test]
    fn test_select_order_by() {
        let expected_sql = "SELECT * FROM `musti` ORDER BY `foo`, `baz` ASC, `bar` DESC LIMIT -1";
        let query = Select::from("musti")
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
        let query = Select::from(("cat", "musti")).column("paw").column("nose");
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(Vec::<ParameterizedValue>::new(), params);
    }

    #[test]
    fn test_select_where_equals() {
        let expected = expected_values(
            "SELECT * FROM `naukio` WHERE `word` = ? LIMIT -1",
            vec!["meow"],
        );

        let query = Select::from("naukio").so_that("word".equals("meow"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_select_where_like() {
        let expected = expected_values(
            "SELECT * FROM `naukio` WHERE `word` LIKE ? LIMIT -1",
            vec!["%meow%"],
        );

        let query = Select::from("naukio").so_that("word".like("meow"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_select_where_not_like() {
        let expected = expected_values(
            "SELECT * FROM `naukio` WHERE `word` NOT LIKE ? LIMIT -1",
            vec!["%meow%"],
        );

        let query = Select::from("naukio").so_that("word".not_like("meow"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_select_where_begins_with() {
        let expected = expected_values(
            "SELECT * FROM `naukio` WHERE `word` LIKE ? LIMIT -1",
            vec!["meow%"],
        );

        let query = Select::from("naukio").so_that("word".begins_with("meow"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_select_where_not_begins_with() {
        let expected = expected_values(
            "SELECT * FROM `naukio` WHERE `word` NOT LIKE ? LIMIT -1",
            vec!["meow%"],
        );

        let query = Select::from("naukio").so_that("word".not_begins_with("meow"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_select_where_ends_into() {
        let expected = expected_values(
            "SELECT * FROM `naukio` WHERE `word` LIKE ? LIMIT -1",
            vec!["%meow"],
        );

        let query = Select::from("naukio").so_that("word".ends_into("meow"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_select_where_not_ends_into() {
        let expected = expected_values(
            "SELECT * FROM `naukio` WHERE `word` NOT LIKE ? LIMIT -1",
            vec!["%meow"],
        );

        let query = Select::from("naukio").so_that("word".not_ends_into("meow"));
        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected.0, sql);
        assert_eq!(expected.1, params);
    }

    #[test]
    fn test_select_and() {
        let expected_sql =
            "SELECT * FROM `naukio` WHERE ((`word` = ? AND `age` < ?) AND `paw` = ?) LIMIT -1";

        let expected_params = vec![
            ParameterizedValue::Text(String::from("meow")),
            ParameterizedValue::Integer(10),
            ParameterizedValue::Text(String::from("warm")),
        ];

        let conditions = "word"
            .equals("meow")
            .and("age".less_than(10))
            .and("paw".equals("warm"));

        let query = Select::from("naukio").so_that(conditions);

        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(expected_params, params);
    }

    #[test]
    fn test_select_and_different_execution_order() {
        let expected_sql =
            "SELECT * FROM `naukio` WHERE (`word` = ? AND (`age` < ? AND `paw` = ?)) LIMIT -1";

        let expected_params = vec![
            ParameterizedValue::Text(String::from("meow")),
            ParameterizedValue::Integer(10),
            ParameterizedValue::Text(String::from("warm")),
        ];

        let conditions = "word"
            .equals("meow")
            .and("age".less_than(10).and("paw".equals("warm")));

        let query = Select::from("naukio").so_that(conditions);

        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(expected_params, params);
    }

    #[test]
    fn test_select_or() {
        let expected_sql =
            "SELECT * FROM `naukio` WHERE ((`word` = ? OR `age` < ?) AND `paw` = ?) LIMIT -1";

        let expected_params = vec![
            ParameterizedValue::Text(String::from("meow")),
            ParameterizedValue::Integer(10),
            ParameterizedValue::Text(String::from("warm")),
        ];

        let conditions = "word"
            .equals("meow")
            .or("age".less_than(10))
            .and("paw".equals("warm"));

        let query = Select::from("naukio").so_that(conditions);

        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(expected_params, params);
    }

    #[test]
    fn test_select_negation() {
        let expected_sql =
            "SELECT * FROM `naukio` WHERE (NOT ((`word` = ? OR `age` < ?) AND `paw` = ?)) LIMIT -1";

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

        let query = Select::from("naukio").so_that(conditions);

        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(expected_params, params);
    }

    #[test]
    fn test_with_raw_condition_tree() {
        let expected_sql =
            "SELECT * FROM `naukio` WHERE (NOT ((`word` = ? OR `age` < ?) AND `paw` = ?)) LIMIT -1";

        let expected_params = vec![
            ParameterizedValue::Text(String::from("meow")),
            ParameterizedValue::Integer(10),
            ParameterizedValue::Text(String::from("warm")),
        ];

        let conditions = ConditionTree::not(ConditionTree::and(
            ConditionTree::or("word".equals("meow"), "age".less_than(10)),
            "paw".equals("warm"),
        ));

        let query = Select::from("naukio").so_that(conditions);

        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(expected_params, params);
    }

    #[test]
    fn test_simple_inner_join() {
        let expected_sql =
            "SELECT * FROM `users` INNER JOIN `posts` ON `users`.`id` = `posts`.`user_id` LIMIT -1";

        let query = Select::from("users")
            .inner_join("posts".on(("users", "id").equals(Column::from(("posts", "user_id")))));
        let (sql, _) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_additional_condition_inner_join() {
        let expected_sql =
            "SELECT * FROM `users` INNER JOIN `posts` ON (`users`.`id` = `posts`.`user_id` AND `posts`.`published` = ?) LIMIT -1";

        let query = Select::from("users").inner_join(
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
            "SELECT * FROM `users` LEFT OUTER JOIN `posts` ON `users`.`id` = `posts`.`user_id` LIMIT -1";

        let query = Select::from("users").left_outer_join(
            "posts".on(("users", "id").equals(Column::from(("posts", "user_id")))),
        );
        let (sql, _) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
    }

    #[test]
    fn test_additional_condition_left_join() {
        let expected_sql =
            "SELECT * FROM `users` LEFT OUTER JOIN `posts` ON (`users`.`id` = `posts`.`user_id` AND `posts`.`published` = ?) LIMIT -1";

        let query = Select::from("users").left_outer_join(
            "posts".on(("users", "id")
                .equals(Column::from(("posts", "user_id")))
                .and(("posts", "published").equals(true))),
        );

        let (sql, params) = Sqlite::build(query);

        assert_eq!(expected_sql, sql);
        assert_eq!(vec![ParameterizedValue::Boolean(true),], params);
    }
}
