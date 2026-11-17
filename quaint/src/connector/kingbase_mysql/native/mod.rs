//! Native KingbaseES connector for the MySQL-compatible provider.
//!
//! Kingbase speaks the PostgreSQL wire protocol, but this connector is kept
//! separate from the PostgreSQL connector so provider-specific URL routing and
//! value conversion do not change the existing PostgreSQL path.

mod column_type;
mod conversion;
mod error;

use column_type::column_type_from_column;

use crate::{
    ast::{Query, Value},
    connector::{
        ColumnType, DescribedColumn, DescribedParameter, DescribedQuery, IsolationLevel, ResultSet, Transaction,
        queryable::*, timeout, trace,
    },
    error::{Error, ErrorKind, NativeErrorKind},
    visitor::KingbaseMysql as KingbaseMysqlVisitor,
};
use async_trait::async_trait;
use futures::{Future, StreamExt, future::FutureExt};
use kingbase_tokio_postgres::{
    Config,
    config::{ChannelBinding, LoadBalanceHosts, SslMode, SslNegotiation, TargetSessionAttrs},
};
use native_tls::{Certificate, Identity, TlsConnector};
use percent_encoding::percent_decode;
use std::{
    collections::HashMap,
    fs,
    sync::{
        RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::task::JoinHandle;
use tracing_futures::WithSubscriber;

use super::{
    KingbaseMysqlUrl,
    url::{SslAcceptMode, SslParams},
};
use kingbase_postgres_native_tls::MakeTlsConnector;
use kingbase_tokio_postgres::{Client, Statement, types::Type};

const DB_SYSTEM_NAME: &str = "kingbase";

const INDEX_NAMES_SQL: &str = "SELECT n.nspname, c.relname, d.description \
    FROM sys_catalog.sys_class c \
    JOIN sys_catalog.sys_namespace n ON n.oid=c.relnamespace \
    JOIN sys_catalog.sys_description d ON d.objoid=c.oid AND d.objsubid=0 \
    AND d.classoid=(SELECT classid FROM sys_catalog.sys_get_object_address(\
        'view', ARRAY['sys_catalog','sys_class'], ARRAY[]::text[])) \
    WHERE c.relkind IN ('i','I') AND d.description LIKE 'prisma:logical-index:%'";

/// A native Kingbase connection using the Kingbase MySQL SQL visitor.
pub struct KingbaseMysql {
    client: Client,
    handle: JoinHandle<()>,
    socket_timeout: Option<Duration>,
    pg_bouncer: bool,
    is_healthy: AtomicBool,
    logical_index_names: RwLock<HashMap<(String, String), String>>,
    transaction_active: AtomicBool,
}

impl KingbaseMysqlUrl {
    /// Build the native driver's configuration explicitly, as the PostgreSQL connector does.
    pub(crate) fn to_config(&self) -> crate::Result<Config> {
        // Hostless URLs could not receive the Kingbase default port in the
        // previous implementation; keep rejecting them instead of changing routing.
        if !self.url.has_host() {
            return Err(Error::builder(ErrorKind::DatabaseUrlIsInvalid("missing Kingbase URL host".into())).build());
        }
        let mut config = Config::new();
        let decode = |value: &str| {
            percent_decode(value.as_bytes())
                .decode_utf8()
                .map(|value| value.into_owned())
                .map_err(|_| {
                    Error::builder(ErrorKind::DatabaseUrlIsInvalid(
                        "invalid UTF-8 in connection URL".into(),
                    ))
                    .build()
                })
        };

        if !self.url.username().is_empty() || self.url.password().is_some() {
            config.user(decode(self.url.username())?);
        }
        if let Some(password) = self.url.password() {
            config.password(percent_decode(password.as_bytes()).collect::<Vec<_>>());
        }
        if let Some(host) = self.url.host_str().filter(|host| !host.is_empty()) {
            let hosts = host.split(',').collect::<Vec<_>>();
            for (index, host) in hosts.iter().enumerate() {
                let host = host
                    .strip_prefix('[')
                    .and_then(|host| host.strip_suffix(']'))
                    .unwrap_or(host);
                let host = percent_decode(host.as_bytes()).collect::<Vec<_>>();
                #[cfg(unix)]
                if host.first() == Some(&b'/') {
                    use std::os::unix::ffi::OsStrExt;
                    config.host_path(std::ffi::OsStr::from_bytes(&host));
                } else {
                    config.host(std::str::from_utf8(host.as_slice()).map_err(|_| {
                        Error::builder(ErrorKind::DatabaseUrlIsInvalid("invalid UTF-8 in host".into())).build()
                    })?);
                }
                #[cfg(not(unix))]
                {
                    let host = std::str::from_utf8(&host).map_err(|_| {
                        Error::builder(ErrorKind::DatabaseUrlIsInvalid("invalid UTF-8 in host".into())).build()
                    })?;
                    // The driver's non-Unix host parser also splits decoded commas.
                    for host in host.split(',') {
                        config.host(host);
                    }
                }
                // Preserve the driver's ordering for comma-separated authority hosts.
                config.port(if index + 1 == hosts.len() { self.port() } else { 5432 });
            }
        }
        if let Some(database) = self.dbname() {
            config.dbname(decode(database)?);
        }

        let params = &self.query_params;
        if let Some(user) = &params.user {
            config.user(user);
        }
        if let Some(password) = &params.password.0 {
            config.password(password);
        }
        if let Some(database) = &params.dbname {
            config.dbname(database);
        }
        if let Some(options) = &params.options {
            config.options(options);
        }
        if let Some(application_name) = &params.application_name {
            config.application_name(application_name);
        }
        for host in &params.hosts {
            #[cfg(unix)]
            config.host(host);
            #[cfg(not(unix))]
            for host in host.split(',') {
                config.host(host);
            }
        }
        for address in &params.hostaddrs {
            config.hostaddr(*address);
        }
        for port in &params.ports {
            config.port(*port);
        }
        if let Some(timeout) = params.driver_connect_timeout {
            config.connect_timeout(timeout);
        }
        if let Some(timeout) = params.tcp_user_timeout {
            config.tcp_user_timeout(timeout);
        }
        if let Some(keepalives) = params.keepalives {
            config.keepalives(keepalives);
        }
        if let Some(timeout) = params.keepalives_idle {
            config.keepalives_idle(timeout);
        }
        if let Some(timeout) = params.keepalives_interval {
            config.keepalives_interval(timeout);
        }
        if let Some(retries) = params.keepalives_retries {
            config.keepalives_retries(retries);
        }
        if let Some(value) = &params.ssl_mode {
            config.ssl_mode(match value.as_str() {
                "disable" => SslMode::Disable,
                "require" => SslMode::Require,
                _ => SslMode::Prefer,
            });
        }
        if let Some(value) = &params.ssl_negotiation {
            config.ssl_negotiation(match value.as_str() {
                "direct" => SslNegotiation::Direct,
                _ => SslNegotiation::Postgres,
            });
        }
        if let Some(value) = &params.target_session_attrs {
            config.target_session_attrs(match value.as_str() {
                "read-write" => TargetSessionAttrs::ReadWrite,
                "read-only" => TargetSessionAttrs::ReadOnly,
                _ => TargetSessionAttrs::Any,
            });
        }
        if let Some(value) = &params.channel_binding {
            config.channel_binding(match value.as_str() {
                "disable" => ChannelBinding::Disable,
                "require" => ChannelBinding::Require,
                _ => ChannelBinding::Prefer,
            });
        }
        if let Some(value) = &params.load_balance_hosts {
            config.load_balance_hosts(match value.as_str() {
                "random" => LoadBalanceHosts::Random,
                _ => LoadBalanceHosts::Disable,
            });
        }
        config.pgbouncer_mode(self.pg_bouncer());
        let schema = self.schema().unwrap_or(super::DEFAULT_KINGBASE_MYSQL_SCHEMA);
        config.search_path(format!("\"{}\"", schema.replace('"', "\"\"")));
        Ok(config)
    }
}

impl KingbaseMysql {
    pub async fn new(url: KingbaseMysqlUrl) -> crate::Result<Self> {
        let config = url.to_config()?;
        let tls = make_tls_connector(url.ssl_params())?;
        let (client, connection) = timeout::connect(url.connect_timeout(), config.connect(tls)).await?;

        let handle = tokio::spawn(
            connection
                .map(|result| {
                    if let Err(error) = result {
                        tracing::error!("Error in Kingbase connection: {error:?}");
                    }
                })
                .with_current_subscriber(),
        );

        Ok(Self {
            client,
            handle,
            socket_timeout: url.socket_timeout(),
            pg_bouncer: url.pg_bouncer(),
            is_healthy: AtomicBool::new(true),
            logical_index_names: RwLock::new(HashMap::new()),
            transaction_active: AtomicBool::new(false),
        })
    }

    async fn refresh_index_names(&self) {
        // Error decoration must never replace the original database error.
        if let Ok(rows) = timeout::socket(self.socket_timeout, self.client.query(INDEX_NAMES_SQL, &[])).await {
            let mut names = self.logical_index_names.write().unwrap();
            names.clear();
            for row in rows {
                let description: &str = row.get(2);
                if let Some(logical) = description
                    .strip_prefix("prisma:logical-index:")
                    .filter(|s| !s.is_empty())
                {
                    names.insert((row.get(0), row.get(1)), logical.to_owned());
                }
            }
        }
    }

    async fn map_driver_error(&self, error: kingbase_tokio_postgres::Error) -> Error {
        if let Some(db_error) = error.as_db_error()
            && db_error.code().code() == "23505"
            && let (Some(schema), Some(index)) = (db_error.schema(), db_error.constraint())
            && index.starts_with("_prisma_idx_")
        {
            let key = (schema.to_owned(), index.to_owned());
            if !self.transaction_active.load(Ordering::SeqCst) {
                // An autocommit connection may outlive migrations on another session.
                self.refresh_index_names().await;
            }
            let logical = self.logical_index_names.read().unwrap().get(&key).cloned();
            if let Some(logical) = logical {
                let mut mapped = super::error::KingbaseError::from(db_error);
                mapped.constraint = Some(logical);
                return mapped.into();
            }
        }
        error.into()
    }

    async fn perform_io<F, T>(&self, fut: F) -> crate::Result<T>
    where
        F: Future<Output = Result<T, kingbase_tokio_postgres::Error>>,
    {
        // Time out the user operation separately so error decoration cannot
        // replace an already-received constraint violation with a timeout.
        let result = timeout::socket(self.socket_timeout, async { Ok::<_, Error>(fut.await) }).await?;
        let result = match result {
            Ok(value) => Ok(value),
            Err(error) => Err(self.map_driver_error(error).await),
        };
        match result {
            Err(error) if error.is_closed() => {
                self.is_healthy.store(false, Ordering::SeqCst);
                Err(error)
            }
            result => result,
        }
    }

    fn check_bind_variables_len(&self, params: &[Value<'_>]) -> crate::Result<()> {
        if params.len() > i16::MAX as usize {
            Err(Error::builder(ErrorKind::QueryInvalidInput(format!(
                "too many bind variables in prepared statement, expected maximum of {}, received {}",
                i16::MAX,
                params.len()
            )))
            .build())
        } else {
            Ok(())
        }
    }

    async fn prepare_statement(
        &self,
        sql: &str,
        params: &[Value<'_>],
        types: Option<&[Type]>,
    ) -> crate::Result<Statement> {
        let mut statement = self
            .perform_io(self.client.prepare_typed(sql, types.unwrap_or_default()))
            .await?;
        if types.is_none() {
            // AST numbers must not become text operands in JSON comparisons or
            // SUM. Preserve concrete column types (BIT and temporal types in
            // particular), and refine only numeric parameters inferred as TEXT.
            let mut refined = statement.params().to_vec();
            let mut changed = false;
            for (parameter, typ) in params.iter().zip(&mut refined) {
                if *typ != Type::TEXT || parameter.is_null() {
                    continue;
                }
                *typ = match &parameter.typed {
                    crate::ast::ValueType::Int32(_) => Type::INT4,
                    crate::ast::ValueType::Int64(_) => Type::INT8,
                    // MySQL evaluates FLOAT query parameters as doubles.
                    crate::ast::ValueType::Float(_) | crate::ast::ValueType::Double(_) => Type::FLOAT8,
                    crate::ast::ValueType::Numeric(_) => Type::NUMERIC,
                    _ => continue,
                };
                changed = true;
            }
            if changed {
                statement = self.perform_io(self.client.prepare_typed(sql, &refined)).await?;
            }
        }
        for typ in statement
            .params()
            .iter()
            .chain(statement.columns().iter().map(|column| column.type_()))
        {
            column_type::ensure_supported_type(typ)?;
        }

        if statement.params().len() != params.len() {
            return Err(Error::builder(ErrorKind::IncorrectNumberOfParameters {
                expected: statement.params().len(),
                actual: params.len(),
            })
            .build());
        }

        Ok(statement)
    }

    async fn query_raw_impl(
        &self,
        sql: &str,
        params: &[Value<'_>],
        types: Option<&[Type]>,
    ) -> crate::Result<ResultSet> {
        self.check_bind_variables_len(params)?;
        let converted_params = conversion::conv_params(params)?;
        let statement = self.prepare_statement(sql, params, types).await?;
        let mut rows = Box::pin(
            self.perform_io(self.client.query_raw(&statement, converted_params))
                .await?,
        );

        let names = statement
            .columns()
            .iter()
            .map(|column| column.name().to_owned())
            .collect::<Vec<_>>();
        let types = statement
            .columns()
            .iter()
            .map(column_type_from_column)
            .collect::<Vec<_>>();
        let mut result = ResultSet::new(names, types, Vec::new());

        while let Some(row) = rows.next().await {
            let row = match row {
                Ok(row) => row,
                Err(error) => return Err(self.map_driver_error(error).await),
            };
            result.rows.push(row.get_result_row()?);
        }

        Ok(result)
    }

    /// Closes the connection and waits for its background task.
    pub async fn close(self) {
        drop(self.client);
        self.handle.await.expect("Kingbase connection task panicked");
    }
}

fn make_tls_connector(ssl_params: &SslParams) -> crate::Result<MakeTlsConnector> {
    let mut tls_builder = TlsConnector::builder();

    if let Some(certificate_file) = &ssl_params.certificate_file {
        let certificate = fs::read(certificate_file).map_err(|error| {
            Error::builder(ErrorKind::Native(NativeErrorKind::TlsError {
                message: format!("cert file not found ({error})"),
            }))
            .build()
        })?;
        let certificate = Certificate::from_pem(&certificate).map_err(|error| {
            Error::builder(ErrorKind::Native(NativeErrorKind::TlsError {
                message: error.to_string(),
            }))
            .build()
        })?;
        tls_builder.add_root_certificate(certificate);
    }

    tls_builder.danger_accept_invalid_certs(ssl_params.ssl_accept_mode == SslAcceptMode::AcceptInvalidCerts);

    if let Some(identity_file) = &ssl_params.identity_file {
        let identity = fs::read(identity_file).map_err(|error| {
            Error::builder(ErrorKind::Native(NativeErrorKind::TlsError {
                message: format!("identity file not found ({error})"),
            }))
            .build()
        })?;
        let password = ssl_params.identity_password.0.as_deref().unwrap_or("");
        let identity = Identity::from_pkcs12(&identity, password).map_err(|error| {
            Error::builder(ErrorKind::Native(NativeErrorKind::TlsError {
                message: error.to_string(),
            }))
            .build()
        })?;
        tls_builder.identity(identity);
    }

    tls_builder
        .build()
        .map_err(|error| {
            Error::builder(ErrorKind::Native(NativeErrorKind::TlsError {
                message: error.to_string(),
            }))
            .build()
        })
        .map(MakeTlsConnector::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connector::Queryable, single::Quaint, tests::test_api::kingbase_mysql::CONN_STR};

    #[tokio::test]
    async fn r3_tinyint_parameters_reject_overflow_without_inserting_rows() {
        let connection = Quaint::new(CONN_STR.as_str()).await.unwrap();
        let table = format!("r3_tinyint_{}", uuid::Uuid::new_v4().simple());
        connection
            .raw_cmd(&format!("CREATE TEMPORARY TABLE `{table}` (value TINYINT)"))
            .await
            .unwrap();
        let insert = format!("INSERT INTO `{table}` VALUES (?)");
        let select = format!("SELECT value FROM `{table}`");

        for integer in [i32::MIN, -129, 128, 255, 256, i32::MAX] {
            assert!(
                connection.execute_raw(&insert, &[Value::int32(integer)]).await.is_err(),
                "out-of-range value {integer} was accepted"
            );
            let result = connection.query_raw(&select, &[]).await.unwrap();
            assert!(result.is_empty(), "out-of-range value {integer} inserted a row");
        }

        for integer in [-128, -1, 0, 1, 127] {
            connection.execute_raw(&insert, &[Value::int32(integer)]).await.unwrap();
            let result = connection.query_raw(&select, &[]).await.unwrap();
            assert_eq!(result.len(), 1);
            assert_eq!(result.first().unwrap()[0].as_i32(), Some(integer));
            connection.raw_cmd(&format!("DELETE FROM `{table}`")).await.unwrap();
        }

        connection.execute_raw(&insert, &[Value::null_int32()]).await.unwrap();
        let result = connection.query_raw(&select, &[]).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(result.first().unwrap()[0].is_null());
    }

    #[tokio::test]
    async fn r3_bit_byte_parameters_preserve_non_byte_aligned_values() {
        let connection = Quaint::new(CONN_STR.as_str()).await.unwrap();
        connection.raw_cmd("BEGIN").await.unwrap();
        let outcome = async {
            for (width, bytes, text) in [
                (7, vec![1], "01"),
                (8, vec![0xaa], "aa"),
                (9, vec![1, 1], "0101"),
                (17, vec![1, 1, 1], "010101"),
                (64, vec![0xff; 8], "ffffffffffffffff"),
            ] {
                let table = format!("r3_bits_{}", uuid::Uuid::new_v4().simple());
                connection
                    .raw_cmd(&format!(
                        "CREATE TEMPORARY TABLE `{table}`(id INT PRIMARY KEY,value BIT({width}))"
                    ))
                    .await?;
                connection
                    .execute_raw(
                        &format!("INSERT INTO `{table}` VALUES(1,?)"),
                        &[Value::bytes(bytes.clone())],
                    )
                    .await?;
                // Check binding independently of the driver's BIT result codec.
                let result = connection
                    .query_raw(&format!("SELECT HEX(value) FROM `{table}`"), &[])
                    .await?;
                assert_eq!(result.first().unwrap()[0].as_str().unwrap().to_ascii_lowercase(), text);
                if width >= 8 {
                    let result = connection
                        .query_raw(&format!("SELECT value FROM `{table}`"), &[])
                        .await?;
                    assert_eq!(result.first().unwrap()[0].as_bytes(), Some(bytes.as_slice()));
                }
            }
            Ok::<(), Error>(())
        }
        .await;
        connection.raw_cmd("ROLLBACK").await.unwrap();
        outcome.unwrap();
    }

    #[tokio::test]
    async fn r3_untyped_single_byte_bit_parameters_preserve_boolean_compatibility() {
        let connection = Quaint::new(CONN_STR.as_str()).await.unwrap();
        for width in [1, 7, 8] {
            let table = format!("r3_bit_boolean_{}", uuid::Uuid::new_v4().simple());
            connection
                .raw_cmd(&format!("CREATE TEMPORARY TABLE `{table}` (value BIT({width}))"))
                .await
                .unwrap();
            for byte in [0, 1] {
                connection
                    .execute_raw(
                        &format!("INSERT INTO `{table}` VALUES (?)"),
                        &[Value::bytes(vec![byte])],
                    )
                    .await
                    .unwrap();
                let result = connection
                    .query_raw(&format!("SELECT value FROM `{table}`"), &[])
                    .await
                    .unwrap();
                let value = &result.first().unwrap()[0];
                if width == 1 {
                    assert_eq!(value.as_bool(), Some(byte == 1));
                } else {
                    assert_eq!(value.as_bytes(), Some([byte].as_slice()));
                }
                connection.raw_cmd(&format!("DELETE FROM `{table}`")).await.unwrap();
            }
        }
    }

    #[tokio::test]
    async fn r3_bit_parameters_use_native_width_for_zero_and_boundary_values() {
        use crate::ast::{Insert, TypeDataLength};

        let connection = Quaint::new(CONN_STR.as_str()).await.unwrap();
        for width in [7_u16, 8, 9, 17, 64] {
            let table = format!("r3_bit_width_{}", uuid::Uuid::new_v4().simple());
            connection
                .raw_cmd(&format!("CREATE TEMPORARY TABLE `{table}` (value BIT({width}))"))
                .await
                .unwrap();
            let max = if width == 64 { u64::MAX } else { (1_u64 << width) - 1 };
            let mut values = vec![None, Some(0), Some(1), Some(max)];
            if width == 64 {
                values.push(Some(420666));
            }
            for number in values {
                let expected = number.map(|number| number.to_be_bytes()[8 - usize::from(width.div_ceil(8))..].to_vec());
                let parameter = match &expected {
                    Some(bytes) => Value::bytes(bytes.clone()),
                    None => Value::null_bytes(),
                }
                .with_native_column_type(Some(("BIT", Some(TypeDataLength::Constant(width)))));

                for path in ["query", "execute", "raw", "typed_raw"] {
                    let insert = Insert::single_into(table.as_str()).value("value", parameter.clone());
                    let sql = format!("INSERT INTO `{table}` (value) VALUES (?)");
                    match path {
                        "query" => {
                            connection.query(insert.into()).await.unwrap();
                        }
                        "execute" => {
                            connection.execute(insert.into()).await.unwrap();
                        }
                        "raw" => {
                            connection.execute_raw(&sql, &[parameter.clone()]).await.unwrap();
                        }
                        "typed_raw" => {
                            connection.execute_raw_typed(&sql, &[parameter.clone()]).await.unwrap();
                        }
                        _ => unreachable!(),
                    }
                    let result = connection
                        .query_raw(&format!("SELECT value FROM `{table}`"), &[])
                        .await
                        .unwrap();
                    assert_eq!(result.types(), &[ColumnType::Bytes]);
                    assert_eq!(
                        result.first().unwrap()[0].as_bytes(),
                        expected.as_deref(),
                        "BIT({width}), {number:?}, {path}"
                    );
                    connection.raw_cmd(&format!("DELETE FROM `{table}`")).await.unwrap();
                }
            }
        }
    }

    #[tokio::test]
    async fn logical_index_names_survive_unique_errors_and_transaction_rollback() {
        let namespace = format!("kb_error_{}", uuid::Uuid::new_v4().simple());
        let mut url: url::Url = CONN_STR.parse().unwrap();
        url.query_pairs_mut().append_pair("schema", &namespace);
        let connection = KingbaseMysql::new(KingbaseMysqlUrl::new(url).unwrap()).await.unwrap();
        connection
            .raw_cmd(&format!("CREATE SCHEMA `{namespace}`"))
            .await
            .unwrap();
        let outcome: crate::Result<_> = async {
            connection
                .raw_cmd(
                    "CREATE TABLE logical_error_names(id INT PRIMARY KEY, value INT); \
                CREATE UNIQUE INDEX `_prisma_idx_error_probe` ON logical_error_names(value);\n\
                COMMENT ON INDEX `_prisma_idx_error_probe` IS 'prisma:logical-index:custom''unique'; \
                INSERT INTO logical_error_names VALUES(1,1)",
                )
                .await?;
            let mut errors = Vec::new();
            for transactional in [false, true] {
                let transaction = if transactional {
                    Some(connection.start_transaction(None).await?)
                } else {
                    None
                };
                let client: &dyn Queryable = transaction.as_ref().map(|t| t.as_queryable()).unwrap_or(&connection);
                if transactional {
                    client
                        .execute_raw("INSERT INTO logical_error_names VALUES(3,3)", &[])
                        .await?;
                }
                errors.push(
                    client
                        .query_raw("INSERT INTO logical_error_names VALUES(2,1)", &[])
                        .await
                        .unwrap_err(),
                );
                if let Some(transaction) = transaction {
                    transaction.rollback().await?;
                }
            }
            let count = connection
                .query_raw("SELECT COUNT(*)::INT FROM logical_error_names", &[])
                .await?
                .first()
                .unwrap()[0]
                .as_integer();
            Ok((errors, count))
        }
        .await;
        connection
            .raw_cmd(&format!("DROP SCHEMA `{namespace}` CASCADE"))
            .await
            .unwrap();
        let (errors, count) = outcome.unwrap();
        for error in errors {
            assert!(
                matches!(error.kind(), ErrorKind::UniqueConstraintViolation { constraint: crate::error::DatabaseConstraint::Index(name) } if name == "custom'unique"),
                "{error:?}"
            );
        }
        assert_eq!(count, Some(1));
    }

    #[tokio::test]
    async fn sql_arrays_are_rejected_but_json_arrays_and_in_queries_work() {
        let connection = Quaint::new(CONN_STR.as_str()).await.unwrap();
        for value in [
            Value::array([1, 2]),
            Value::array(Vec::<i32>::new()),
            Value::null_array(),
            Value::null_enum_array(),
        ] {
            let params = [value];
            // Invalid SQL ensures unsupported arguments are rejected before prepare.
            assert!(
                connection
                    .query_raw("not SQL", &params)
                    .await
                    .unwrap_err()
                    .to_string()
                    .contains("Arrays are not supported")
            );
            assert!(
                connection
                    .query_raw_typed("not SQL", &params)
                    .await
                    .unwrap_err()
                    .to_string()
                    .contains("Arrays are not supported")
            );
            assert!(
                connection
                    .execute_raw("not SQL", &params)
                    .await
                    .unwrap_err()
                    .to_string()
                    .contains("Arrays are not supported")
            );
            assert!(
                connection
                    .execute_raw_typed("not SQL", &params)
                    .await
                    .unwrap_err()
                    .to_string()
                    .contains("Arrays are not supported")
            );
        }
        for sql in [
            "SELECT ARRAY[1,2] AS value",
            "SELECT NULL::integer[] AS value",
            "SELECT ARRAY[1] AS value WHERE false",
        ] {
            assert!(matches!(
                connection.query_raw(sql, &[]).await.unwrap_err().kind(),
                ErrorKind::UnsupportedColumnType { .. }
            ));
            assert!(connection.describe_query(sql).await.is_err());
        }
        let json = serde_json::json!([1, "123", null]);
        let result = connection
            .query_raw("SELECT CAST(? AS JSON) AS value", &[Value::json(json.clone())])
            .await
            .unwrap();
        assert_eq!(result.first().unwrap()[0].as_json(), Some(&json));
        let result = connection
            .query_raw("SELECT 2 IN (?,?) AS value", &[Value::int32(1), Value::int32(2)])
            .await
            .unwrap();
        assert_eq!(result.first().unwrap()[0].as_bool(), Some(true));
    }

    #[tokio::test]
    async fn r3_mysql_json_typed_parameters_and_results_roundtrip() {
        let connection = Quaint::new(CONN_STR.as_str()).await.unwrap();
        let table = format!("r3_json_{}", uuid::Uuid::new_v4().simple());
        connection.raw_cmd("BEGIN").await.unwrap();
        let outcome = async {
            connection
                .raw_cmd(&format!(
                    "CREATE TEMPORARY TABLE `{table}` (id INTEGER PRIMARY KEY, value JSON)"
                ))
                .await?;
            let values = [
                serde_json::json!({"n": "123"}),
                serde_json::json!([1, "123", null]),
                serde_json::json!("null"),
                serde_json::json!("123"),
                serde_json::json!(123),
                serde_json::json!(u64::MAX),
                serde_json::json!(true),
                serde_json::json!(false),
                serde_json::Value::Null,
            ];
            for (index, value) in values.iter().enumerate() {
                let parameter = Value::json(value.clone());
                let result = connection
                    .query_raw_typed("SELECT CAST(? AS JSON) AS value", &[parameter.clone()])
                    .await?;
                assert_eq!(result.types(), &[ColumnType::Json]);
                assert_eq!(result.first().unwrap()[0].as_json(), Some(value));
                connection
                    .execute_raw_typed(
                        &format!("INSERT INTO `{table}` VALUES (?,?)"),
                        &[Value::int32(index as i32), parameter],
                    )
                    .await?;
            }
            let rows = connection
                .query_raw(&format!("SELECT value FROM `{table}` ORDER BY id"), &[])
                .await?;
            assert_eq!(rows.types(), &[ColumnType::Json]);
            assert_eq!(rows.len(), values.len());
            for (row, value) in rows.into_iter().zip(&values) {
                assert_eq!(row[0].as_json(), Some(value));
            }
            for (expression, expected) in [
                ("JSON_EXTRACT(value, '$.n')", serde_json::json!("123")),
                ("JSON_ARRAYAGG(value)", serde_json::json!([{"n": "123"}])),
                ("JSON_OBJECTAGG('n', value)", serde_json::json!({"n": {"n": "123"}})),
            ] {
                let result = connection
                    .query_raw(&format!("SELECT {expression} AS value FROM `{table}` WHERE id=0"), &[])
                    .await?;
                assert_eq!(result.types(), &[ColumnType::Json]);
                assert_eq!(result.first().unwrap()[0].as_json(), Some(&expected));
            }
            let sql_null = connection.query_raw("SELECT CAST(NULL AS JSON) AS value", &[]).await?;
            assert_eq!(sql_null.types(), &[ColumnType::Json]);
            assert!(sql_null.first().unwrap()[0].is_null());
            let empty = connection
                .query_raw("SELECT CAST('null' AS JSON) AS value WHERE false", &[])
                .await?;
            assert_eq!(empty.types(), &[ColumnType::Json]);
            assert!(empty.is_empty());
            let described = connection
                .describe_query("SELECT CAST('null' AS JSON) AS value")
                .await?;
            assert_eq!(described.columns[0].typ, ColumnType::Json);
            for sql in [
                "SELECT ARRAY['null'::sys.mysql_json] AS value",
                "SELECT NULL::sys.mysql_json[] AS value",
                "SELECT ARRAY['null'::sys.mysql_json] AS value WHERE false",
            ] {
                assert!(matches!(
                    connection.query_raw(sql, &[]).await.unwrap_err().kind(),
                    ErrorKind::UnsupportedColumnType { .. }
                ));
            }
            Ok::<_, Error>(())
        }
        .await;
        connection.raw_cmd("ROLLBACK").await.unwrap();
        outcome.unwrap();
    }

    #[tokio::test]
    async fn decimal_sum_without_cast_uses_the_inferred_text_parameter() {
        let connection = Quaint::new(CONN_STR.as_str()).await.unwrap();
        let described = connection.describe_query("SELECT SUM(?)").await.unwrap();
        assert_eq!(described.parameters.len(), 1);
        assert_eq!(described.parameters[0].typ, ColumnType::Text);

        for (parameter, expected) in [
            (Value::numeric("1.25".parse().unwrap()), Some(1.25)),
            (Value::numeric("-1.25".parse().unwrap()), Some(-1.25)),
            (Value::numeric("1E-31".parse().unwrap()), Some(1e-31)),
            (Value::null_numeric(), None),
        ] {
            let result = connection.query_raw("SELECT SUM(?)", &[parameter]).await.unwrap();
            assert_eq!(result.types(), &[ColumnType::Double]);
            let row = result.first().unwrap();
            assert_eq!(row[0].as_f64(), expected);
            assert_eq!(row[0].is_null(), expected.is_none());
        }
    }

    #[tokio::test]
    async fn describe_query_infers_a_double_parameter_from_the_column() {
        let connection = Quaint::new(CONN_STR.as_str()).await.unwrap();

        connection
            .raw_cmd("CREATE TEMPORARY TABLE `typed_sql_double` (`value` DOUBLE)")
            .await
            .unwrap();

        let described = connection
            .describe_query("SELECT `value` FROM `typed_sql_double` WHERE `value` < ?")
            .await
            .unwrap();

        assert_eq!(described.parameters.len(), 1);
        // Unlike an unconstrained `SELECT ?`, the comparison supplies enough
        // context for R3 to infer DOUBLE in prepared-statement metadata.
        assert_eq!(described.parameters[0].typ, ColumnType::Double);
    }
}

impl_default_TransactionCapable!(KingbaseMysql);

#[async_trait]
impl Queryable for KingbaseMysql {
    async fn query(&self, query: Query<'_>) -> crate::Result<ResultSet> {
        let (sql, params) = KingbaseMysqlVisitor::build(query)?;
        trace::query(DB_SYSTEM_NAME, &sql, &params, || async {
            self.query_raw_impl(&sql, &params, None).await
        })
        .await
    }

    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        trace::query(DB_SYSTEM_NAME, sql, params, move || async move {
            self.query_raw_impl(sql, params, Some(&[])).await
        })
        .await
    }

    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet> {
        trace::query(DB_SYSTEM_NAME, sql, params, move || async move {
            self.query_raw_impl(sql, params, Some(&conversion::params_to_types(params)))
                .await
        })
        .await
    }

    async fn execute(&self, query: Query<'_>) -> crate::Result<u64> {
        let (sql, params) = KingbaseMysqlVisitor::build(query)?;
        trace::query(DB_SYSTEM_NAME, &sql, &params, || async {
            self.execute_raw_with_types(&sql, &params, None).await
        })
        .await
    }

    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        trace::query(DB_SYSTEM_NAME, sql, params, move || async move {
            self.execute_raw_with_types(sql, params, Some(&[])).await
        })
        .await
    }

    async fn execute_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64> {
        trace::query(DB_SYSTEM_NAME, sql, params, move || async move {
            self.execute_raw_with_types(sql, params, Some(&conversion::params_to_types(params)))
                .await
        })
        .await
    }

    async fn raw_cmd(&self, cmd: &str) -> crate::Result<()> {
        trace::query(DB_SYSTEM_NAME, cmd, &[], move || async move {
            let command = cmd.trim().trim_end_matches(';').trim();
            if command.eq_ignore_ascii_case("BEGIN") {
                // Read before BEGIN: after a constraint error this session cannot
                // run catalog queries until the caller rolls the transaction back.
                self.refresh_index_names().await;
            }
            let names = super::error::index_names_from_sql(cmd);
            let has_index_comments = !names.is_empty();
            if let Err(error) = self.perform_io(self.client.simple_query(cmd)).await {
                if let ErrorKind::UniqueConstraintViolation {
                    constraint: crate::error::DatabaseConstraint::Index(physical),
                } = error.kind()
                    && let Some((_, logical)) = names.iter().find(|(name, _)| name == physical)
                {
                    let mut mapped = Error::builder(ErrorKind::UniqueConstraintViolation {
                        constraint: crate::error::DatabaseConstraint::Index(logical.clone()),
                    });
                    if let Some(code) = error.original_code() {
                        mapped.set_original_code(code);
                    }
                    if let Some(message) = error.original_message() {
                        mapped.set_original_message(message);
                    }
                    return Err(mapped.build());
                }
                return Err(error);
            }
            if command.eq_ignore_ascii_case("BEGIN") {
                self.transaction_active.store(true, Ordering::SeqCst);
            } else if command.eq_ignore_ascii_case("COMMIT") || command.eq_ignore_ascii_case("ROLLBACK") {
                self.transaction_active.store(false, Ordering::SeqCst);
            }
            if has_index_comments {
                self.refresh_index_names().await;
            }
            Ok(())
        })
        .await
    }

    async fn version(&self) -> crate::Result<Option<String>> {
        let result = self.query_raw("SELECT version()", &[]).await?;
        Ok(result
            .first()
            .and_then(|row| row.get("version").and_then(|value| value.to_string())))
    }

    async fn describe_query(&self, sql: &str) -> crate::Result<DescribedQuery> {
        let statement = self.perform_io(self.client.prepare_typed(sql, &[])).await?;
        for typ in statement
            .params()
            .iter()
            .chain(statement.columns().iter().map(|column| column.type_()))
        {
            column_type::ensure_supported_type(typ)?;
        }
        let columns = statement
            .columns()
            .iter()
            .map(|column| DescribedColumn::new_named(column.name(), column_type_from_column(column)).is_nullable(true))
            .collect();
        let parameters = statement
            .params()
            .iter()
            .enumerate()
            .map(|(idx, typ)| DescribedParameter::new_unnamed(idx, ColumnType::from(typ)))
            .collect();

        Ok(DescribedQuery {
            columns,
            parameters,
            enum_names: None,
        })
    }

    fn is_healthy(&self) -> bool {
        self.is_healthy.load(Ordering::SeqCst)
    }

    async fn server_reset_query(&self, tx: &dyn Transaction) -> crate::Result<()> {
        if self.pg_bouncer {
            tx.raw_cmd("DEALLOCATE ALL").await
        } else {
            Ok(())
        }
    }

    async fn set_tx_isolation_level(&self, isolation_level: IsolationLevel) -> crate::Result<()> {
        if matches!(isolation_level, IsolationLevel::Snapshot) {
            return Err(Error::builder(ErrorKind::invalid_isolation_level(&isolation_level)).build());
        }

        self.raw_cmd(&format!("SET TRANSACTION ISOLATION LEVEL {isolation_level}"))
            .await
    }

    fn requires_isolation_first(&self) -> bool {
        false
    }
}

impl KingbaseMysql {
    async fn execute_raw_with_types(
        &self,
        sql: &str,
        params: &[Value<'_>],
        types: Option<&[Type]>,
    ) -> crate::Result<u64> {
        self.check_bind_variables_len(params)?;
        let converted_params = conversion::conv_params(params)?;
        let statement = self.prepare_statement(sql, params, types).await?;

        self.perform_io(self.client.execute(&statement, converted_params.as_slice()))
            .await
    }
}
