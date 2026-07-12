use schema_connector::{ConnectorError, ConnectorResult};
use url::Url;

pub fn parse(connection_string: &str) -> ConnectorResult<Url> {
    let mut url = connection_string.parse().map_err(ConnectorError::url_parse_error)?;
    disable_postgres_statement_cache(&mut url);
    Ok(url)
}

fn disable_postgres_statement_cache(url: &mut Url) {
    let params: Vec<(_, _)> = url.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect();

    url.query_pairs_mut().clear();

    for (k, v) in params {
        if k == "statement_cache_size" {
            url.query_pairs_mut().append_pair("statement_cache_size", "0");
        } else {
            url.query_pairs_mut().append_pair(&k, &v);
        }
    }

    if !url.query_pairs().any(|(k, _)| k == "statement_cache_size") {
        url.query_pairs_mut().append_pair("statement_cache_size", "0");
    }
}
