use deadpool_postgres::{Manager, PoolError};
use regex::Regex;
use serde_json::json;
use std::str::FromStr;
use tokio_postgres::types::ToSql;

use crate::{
    kb::KnowledgeBase,
    query::{SlowQuery, Tag},
};

#[derive(Clone)]
pub struct Stats {
    pool: deadpool_postgres::Pool,
    kb: KnowledgeBase,
}

impl Stats {
    pub fn init(database_url: &str, kb: KnowledgeBase) -> Stats {
        let config = tokio_postgres::Config::from_str(database_url).unwrap();

        let manager_config = deadpool_postgres::ManagerConfig {
            recycling_method: deadpool_postgres::RecyclingMethod::Fast,
        };
        let manager = Manager::from_config(config, tokio_postgres::NoTls, manager_config);
        let pool = deadpool_postgres::Pool::new(manager, num_cpus::get());
        Stats { pool, kb }
    }

    pub async fn slow_queries(&self, threshold: f64, k: i32) -> Result<Vec<SlowQuery>, PoolError> {
        let conn = self.pool.get().await?;
        let stmt = conn
            .prepare(
                r#"
                    SELECT 
                    * 
                    FROM (
                        SELECT
                        mean_exec_time,
                        calls,
                        query
                        FROM pg_stat_statements
                        WHERE query LIKE '%/*%doctor_id%*/'
                        ORDER BY mean_exec_time DESC 
                        LIMIT $2
                    ) as q
                    WHERE q.avg > $1;
                "#,
            )
            .await?;

        let threshold: &(dyn ToSql + Sync) = &threshold;
        let n: &(dyn ToSql + Sync) = &k;
        let rows = conn.query(&stmt, &[threshold, n]).await?;
        let slow_queries = rows
            .into_iter()
            .filter_map(|row| {
                let query: String = row.get("query");
                let mean_exec_time: f64 = row.get("mean_exec_time");
                let calls: u32 = row.get("calls");
                self.hidrate_slow_query(query, calls, mean_exec_time)
            })
            .collect();

        Ok(slow_queries)
    }

    fn hidrate_slow_query(&self, log_query: String, num_executions: u32, mean_exec_time: f64) -> Option<SlowQuery> {
        if let Some(tag) = Self::extract_tag(&log_query) {
            let (sql, prisma_queries) = self.kb.get_tagged(tag);
            Some(SlowQuery {
                sql,
                prisma_queries,
                mean_exec_time,
                num_executions,
                query_plan: "".to_string(),
                additional_info: json!({}),
            })
        } else {
            None
        }
    }

    fn extract_tag(query: &str) -> Option<Tag> {
        // extracts the value of the tag doctor_id inside an sql comment from the query string using a regular expression
        let comment_regex = Regex::new(r"/\*\s*doctor_id:\s?(?P<tag>[a-zA-Z0-9_.-]+).*\*/").unwrap();
        // Find the comment match
        let matches = comment_regex.captures(query);
        matches.map(|captures| captures.name("tag").unwrap().as_str().to_string())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_extract_tags() {
        let query = "SELECT * FROM users WHERE id = 1 /* doctor_id: 9a47389815e545676b133bbf6887557d4832563c */";
        let tags = super::Stats::extract_tag(query);
        assert_eq!(tags, Some("9a47389815e545676b133bbf6887557d4832563c".to_string()));

        let query = "SELECT * FROM users WHERE id = 1 /*doctor_id:9a47389815e545676b133bbf6887557d4832563c */";
        let tags = super::Stats::extract_tag(query);
        assert_eq!(tags, Some("9a47389815e545676b133bbf6887557d4832563c".to_string()));

        let query = "SELECT * FROM users WHERE id = 1 /*doctor_id: */";
        let tags = super::Stats::extract_tag(query);
        assert_eq!(tags, None);
    }
}
