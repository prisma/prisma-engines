use deadpool_postgres::{Manager, PoolError};
use log::debug;
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
}

impl Stats {
    pub fn init(database_url: &str) -> Stats {
        let config = tokio_postgres::Config::from_str(database_url).unwrap();

        let manager_config = deadpool_postgres::ManagerConfig {
            recycling_method: deadpool_postgres::RecyclingMethod::Fast,
        };
        let manager = Manager::from_config(config, tokio_postgres::NoTls, manager_config);
        let pool = deadpool_postgres::Pool::new(manager, num_cpus::get());
        Stats { pool }
    }

    pub async fn slow_queries(&self, kb: KnowledgeBase, threshold: f64, k: i64) -> Result<Vec<SlowQuery>, PoolError> {
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
                    WHERE q.mean_exec_time > $1;
                "#,
            )
            .await?;

        let threshold: &(dyn ToSql + Sync) = &threshold;
        let n: &(dyn ToSql + Sync) = &k;
        let rows = conn.query(&stmt, &[threshold, n]).await?;

        let mut slow_queries = Vec::new();
        for row in rows.iter() {
            debug!("Fetching row from slow queries: {:?}", row);
            let query: String = row.get("query");
            let mean_exec_time: f64 = row.get("mean_exec_time");
            let num_executions: i64 = row.get("calls");
            if let Some(record) = self
                .hidrate_slow_query(kb.clone(), query, num_executions, mean_exec_time)
                .await
            {
                debug!("Hidrated slow query: {:?}", record);
                slow_queries.push(record);
            }
        }

        Ok(slow_queries)
    }

    async fn hidrate_slow_query(
        &self,
        kb: KnowledgeBase,
        log_query: String,
        num_executions: i64,
        mean_exec_time: f64,
    ) -> Option<SlowQuery> {
        debug!("Hidrating slow query: {:?}", log_query);

        if let Some(tag) = Self::extract_tag(&log_query) {
            debug!("Fetching from knowledge base for tag: {:?}", tag);

            if let Some((sql, prisma_queries)) = kb.get_tagged(tag) {
                let query_plan = self.explain(&sql).await;

                return Some(SlowQuery {
                    sql,
                    prisma_queries,
                    mean_exec_time,
                    num_executions,
                    query_plan,
                    additional_info: json!({}),
                });
            }
        }

        None
    }

    fn extract_tag(query: &str) -> Option<Tag> {
        // extracts the value of the tag doctor_id inside an sql comment from the query string using a regular expression
        let comment_regex = Regex::new(r"/\*\s*doctor_id:\s?(?P<tag>[a-zA-Z0-9_.-]+).*\*/").unwrap();
        // Find the comment match
        let matches = comment_regex.captures(query);
        matches.map(|captures| captures.name("tag").unwrap().as_str().to_string())
    }

    async fn explain(&self, sql: &str) -> String {
        let conn = self.pool.get().await.unwrap();
        debug!("Explaining query: {:?}", sql);

        let stmt = format!("EXPLAIN (FORMAT JSON) {sql}");

        let rows = conn.query(&stmt, &[]).await.unwrap();
        let row: serde_json::Value = rows[0].get(0);
        serde_json::to_string_pretty(&row).unwrap()
    }

    pub async fn __exec_query(&self, sql: &str) -> Result<(), PoolError> {
        let conn = self.pool.get().await?;
        debug!("Executing query: {:?}", sql);
        conn.execute(sql, &[]).await?;
        Ok(())
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
