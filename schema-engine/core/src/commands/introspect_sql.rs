use crate::json_rpc::types::IntrospectSqlParams;
use schema_connector::{IntrospectSqlQueryInput, IntrospectSqlResult, SchemaConnector};

pub async fn introspect_sql(
    input: IntrospectSqlParams,
    connector: &mut dyn SchemaConnector,
) -> crate::CoreResult<IntrospectSqlResult> {
    let queries: Vec<_> = input
        .queries
        .into_iter()
        .map(|q| IntrospectSqlQueryInput {
            name: q.name,
            source: q.source,
        })
        .collect();

    let mut parsed_queries = Vec::with_capacity(queries.len());

    for q in queries {
        let parsed_query = connector.introspect_sql(q).await?;

        parsed_queries.push(parsed_query);
    }

    Ok(IntrospectSqlResult {
        queries: parsed_queries,
    })
}
