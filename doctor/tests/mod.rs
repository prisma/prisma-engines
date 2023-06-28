use deadpool_postgres::PoolError;
use doctor_lib::{
    pg,
    query::{SlowQuery, SubmittedQueryInfo},
};
use reqwest::Response;
use serde_json::json;
use std::{env, process, vec};

const TEST_PORT: &str = "8081";

struct Cleaner<'a> {
    p: &'a mut std::process::Child,
}
impl<'a> Drop for Cleaner<'a> {
    fn drop(&mut self) {
        self.p.kill().expect("Failed to kill doctor_process");
    }
}

#[actix_rt::test]
async fn end_to_end() {
    // Spawn doctor
    let mut doctor = doctor_cmd(TEST_PORT)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();

    // cleaner will kill the process when the the function is done
    let _cleaner = Cleaner { p: &mut doctor };

    // wait for the process to start.
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Create parameterized queries and its corresponding prisma query
    let tuples: Vec<(&str, &str, &str)> = vec![
        (
            "select count(*) from users where username like '%coo-coo%'",
            "prisma.user.count({ where: {username: { contains: 'coo-coo' }}",
            "3da541559918a808c2402bba5012f6c60b27661c",
        ),
        (
            "select count(*) from users where username like '%foo-bar%'",
            "prisma.user.count({ where: {username: { contains: 'foo-bar' }}",
            "3da541559918a808c2402bba5012f6c60b27661c",
        ),
    ];

    clear_stats().await.expect("Failed to clear stats");

    // Simulate the work done by the query-engine: run the raw query, submit the query info to doctor.
    for (sql, query, tag) in tuples.clone() {
        let tagged_sql = format!("{} /* doctor_id: {} */", sql, tag);
        run_pg_query(&tagged_sql)
            .await
            .expect(format!("Failed to run query: {}", &tagged_sql).as_str());
        submit_query_info(sql, query, tag)
            .await
            .expect("Failed to submit query info");
    }

    // Check that doctor reports slow queries
    let slow_queries = get_slow_queries().await.expect("Failed to get slow queries");
    assert_eq!(slow_queries.len(), 1);

    println!(
        "Slow queries:\n {}",
        serde_json::to_string_pretty(&slow_queries).unwrap()
    );

    let first = slow_queries.first().unwrap();
    assert_eq!(first.num_executions, 2);
    assert_eq!(first.prisma_queries.len(), 2);
    assert!(first.prisma_queries.contains(&tuples[0].1.to_string()));
    assert!(first.prisma_queries.contains(&tuples[1].1.to_string()));
}

async fn clear_stats() -> Result<Response, reqwest::Error> {
    reqwest::Client::new()
        .post(format!("http://localhost:{}/clear-stats", TEST_PORT))
        .send()
        .await
}

async fn get_slow_queries() -> Result<Vec<SlowQuery>, reqwest::Error> {
    reqwest::get(format!(
        "http://localhost:{}/slow-queries?threshold=0.0&k=10",
        TEST_PORT
    ))
    .await?
    .json::<Vec<SlowQuery>>()
    .await
}

async fn submit_query_info(raw_query: &str, prisma_query: &str, tag: &str) -> Result<Response, reqwest::Error> {
    let query_info = SubmittedQueryInfo {
        raw_query: raw_query.to_string(),
        prisma_query: prisma_query.to_string(),
        tag: tag.to_string(),
    };

    reqwest::Client::new()
        .post(format!("http://localhost:{}/submit-query", TEST_PORT))
        .body(json!(query_info).to_string())
        .header("Content-Type", "application/json")
        .send()
        .await
}

async fn run_pg_query(tagged_sql: &str) -> Result<(), PoolError> {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => String::from("postgresql://postgres:prisma@localhost:5432"),
    };

    let conn = pg::Stats::init(&database_url);
    conn.__exec_query(&tagged_sql).await
}

pub(crate) fn doctor_cmd(port: &str) -> process::Command {
    let name = "doctor";
    let env_var = format!("CARGO_BIN_EXE_{}", name);
    let doctor_path = std::env::var_os(env_var).map(|p| p.into()).unwrap_or_else(|| {
        env::current_exe()
            .ok()
            .map(|mut path| {
                path.pop();
                if path.ends_with("deps") {
                    path.pop();
                }
                path
            })
            .unwrap()
            .join(format!("{}{}", name, env::consts::EXE_SUFFIX))
    });

    let mut cmd = std::process::Command::new(doctor_path);
    cmd.env("PORT", port);
    cmd
}
