use crate::{
    logger::Logger,
    migrations::{
        detect_failed_migrations, execute_migration_script, list_migration_dir, list_migrations,
        record_migration_started, MigrationDirectory,
    },
};
use once_cell::sync::Lazy;
use query_core::{
    protocol::EngineProtocol,
    schema::{self},
    telemetry, TransactionOptions, TxId,
};
use request_handlers::{load_executor, RequestBody, RequestHandler};
use serde_json::json;
use std::{
    env,
    ffi::{c_char, c_int, CStr, CString},
    path::{Path, PathBuf},
    ptr::null_mut,
    sync::Arc,
};
use tokio::{
    runtime::{self, Runtime},
    sync::RwLock,
};
use tracing::{field, instrument::WithSubscriber, level_filters::LevelFilter, Instrument};

use query_engine_common::Result;
use query_engine_common::{
    engine::{stringify_env_values, ConnectedEngine, ConnectedEngineNative, EngineBuilder, EngineBuilderNative, Inner},
    error::ApiError,
};
use request_handlers::ConnectorKind;

// The query engine code is async by nature, however the C API does not function with async functions
// This runtime is here to allow the C API to block_on it and return the responses in a sync manner
static RUNTIME: Lazy<Runtime> = Lazy::new(|| runtime::Builder::new_multi_thread().enable_all().build().unwrap());

// C-like return codes
#[no_mangle]
pub static PRISMA_OK: i32 = 0;
#[no_mangle]
pub static PRISMA_UNKNOWN_ERROR: i32 = 1;
#[no_mangle]
pub static PRISMA_MISSING_POINTER: i32 = 2;

/// This struct holds an instance of the prisma query engine
/// You can instanciate as many as you want
pub struct QueryEngine {
    inner: RwLock<Inner>,
    base_path: Option<String>,
    logger: Logger,
    url: String,
}

#[repr(C)]
pub struct ConstructorOptionsNative {
    pub config_dir: *const c_char,
}

/// Parameters defining the construction of an engine.
/// Unlike the Node version, this doesn't support the GraphQL protocol for talking with the prisma/client, since it is
/// deprecated and going forward everything should be done via JSON rpc.
#[repr(C)]
pub struct ConstructorOptions {
    id: *const c_char,
    datamodel: *const c_char,
    // Used on iOS/Android to navigate to the sandboxed app folder to execute all file operations because file systems are sandboxed
    // Take a look at README for a more detailed explanation
    base_path: *const c_char,
    log_level: *const c_char,
    log_queries: bool,
    datasource_overrides: *const c_char,
    env: *const c_char,
    ignore_env_var_errors: bool,
    native: ConstructorOptionsNative,
    log_callback: unsafe extern "C" fn(*const c_char, *const c_char),
}

fn get_cstr_safe(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        let cstr = unsafe { CStr::from_ptr(ptr) };
        Some(String::from_utf8_lossy(cstr.to_bytes()).to_string())
    }
}

fn map_known_error(err: query_core::CoreError) -> Result<String> {
    let user_error: user_facing_errors::Error = err.into();
    let value = serde_json::to_string(&user_error)?;

    Ok(value)
}

fn serialize_api_error(err: ApiError) -> String {
    let user_error: user_facing_errors::Error = err.into();
    serde_json::to_string(&user_error).unwrap()
}

// Struct that holds an internal prisma engine
// the inner prop holds the internal state, it starts as a Builder
// meaning it is not connected to the database
// a call to connect is necessary to start executing queries
impl QueryEngine {
    /// Parse a valid datamodel and configuration to allow connecting later on.
    pub fn new(constructor_options: ConstructorOptions) -> Result<Self> {
        // Create a logs closure that can be passed around and called at any time
        // safe scheduling should be taken care by the code/language/environment calling this C-compatible API
        let engine_id = get_cstr_safe(constructor_options.id).expect("engine id cannot be missing");
        let log_callback_c = constructor_options.log_callback;
        let log_callback = move |msg: String| {
            let id = CString::new(engine_id.clone()).unwrap();
            let c_message = CString::new(msg).unwrap();
            unsafe {
                log_callback_c(id.as_ptr(), c_message.as_ptr());
            }
        };

        let str_env = get_cstr_safe(constructor_options.env).expect("Environment missing");
        let json_env = serde_json::from_str(str_env.as_str()).expect("Environment cannot be parsed");
        let env = stringify_env_values(json_env)?; // we cannot trust anything JS sends us from process.env

        let str_datasource_overrides =
            get_cstr_safe(constructor_options.datasource_overrides).expect("Datesource overrides missing");
        let json_datasource_overrides =
            serde_json::from_str(str_datasource_overrides.as_str()).expect("Datasource overrides cannot be parsed");
        let overrides: Vec<(_, _)> = stringify_env_values(json_datasource_overrides)
            .unwrap()
            .into_iter()
            .collect();

        let datamodel = get_cstr_safe(constructor_options.datamodel).expect("Datamodel must be present");
        let mut schema = psl::validate(datamodel.into());

        let config = &mut schema.configuration;
        config
            .resolve_datasource_urls_query_engine(
                &overrides,
                |key| env.get(key).map(ToString::to_string),
                // constructor_options.ignore_env_var_errors,
                true,
            )
            .map_err(|err| ApiError::conversion(err, schema.db.source_assert_single()))?;
        // extract the url for later use in apply_migrations
        let url = config
            .datasources
            .first()
            .unwrap()
            .load_url(|key| env::var(key).ok())
            .map_err(|err| ApiError::conversion(err, schema.db.source_assert_single()))?;

        schema
            .diagnostics
            .to_result()
            .map_err(|err| ApiError::conversion(err, schema.db.source_assert_single()))?;

        let base_path = get_cstr_safe(constructor_options.base_path);
        match &base_path {
            Some(path) => env::set_current_dir(Path::new(&path)).expect("Could not change directory"),
            _ => tracing::trace!("No base path provided"),
        }

        config
            .validate_that_one_datasource_is_provided()
            .map_err(|errors| ApiError::conversion(errors, schema.db.source_assert_single()))?;

        let engine_protocol = EngineProtocol::Json;

        let config_dir_string = get_cstr_safe(constructor_options.native.config_dir).expect("Config dir is expected");
        let config_dir = PathBuf::from(config_dir_string);

        let builder = EngineBuilder {
            schema: Arc::new(schema),
            engine_protocol,
            native: EngineBuilderNative { config_dir, env },
        };

        let log_level_string = get_cstr_safe(constructor_options.log_level).unwrap();
        let log_level = log_level_string.parse::<LevelFilter>().unwrap();
        let logger = Logger::new(
            constructor_options.log_queries,
            log_level,
            Box::new(log_callback),
            false,
        );

        Ok(Self {
            inner: RwLock::new(Inner::Builder(builder)),
            base_path,
            logger,
            url,
        })
    }

    pub async fn connect(&self, trace: *const c_char) -> Result<()> {
        if let Some(base_path) = self.base_path.as_ref() {
            env::set_current_dir(Path::new(&base_path)).expect("Could not change directory");
        }

        let trace_string = get_cstr_safe(trace).expect("Connect trace is missing");

        let span = tracing::info_span!("prisma:engine:connect");
        let _ = telemetry::helpers::set_parent_context_from_json_str(&span, &trace_string);

        let mut inner = self.inner.write().await;
        let builder = inner.as_builder()?;
        let arced_schema = Arc::clone(&builder.schema);
        let arced_schema_2 = Arc::clone(&builder.schema);

        let engine = async move {
            // We only support one data source & generator at the moment, so take the first one (default not exposed yet).
            let data_source = arced_schema
                .configuration
                .datasources
                .first()
                .ok_or_else(|| ApiError::configuration("No valid data source found"))?;

            let preview_features = arced_schema.configuration.preview_features();

            let executor_fut = async {
                let url = data_source
                    .load_url_with_config_dir(&builder.native.config_dir, |key| {
                        builder.native.env.get(key).map(ToString::to_string)
                    })
                    .map_err(|err| ApiError::Conversion(err, builder.schema.db.source_assert_single().to_owned()))?;
                // This version of the query engine supports connecting via Rust bindings directly
                // support for JS drivers can be added, but I commented it out for now
                let connector_kind = ConnectorKind::Rust {
                    url,
                    datasource: data_source,
                };

                let executor = load_executor(connector_kind, preview_features).await?;
                let connector = executor.primary_connector();

                let conn_span = tracing::info_span!(
                    "prisma:engine:connection",
                    user_facing = true,
                    "db.type" = connector.name(),
                );

                connector.get_connection().instrument(conn_span).await?;

                Result::<_>::Ok(executor)
            };

            let query_schema_span = tracing::info_span!("prisma:engine:schema");
            let query_schema_fut = tokio::runtime::Handle::current()
                .spawn_blocking(move || {
                    let enable_raw_queries = true;
                    schema::build(arced_schema_2, enable_raw_queries)
                })
                .instrument(query_schema_span);

            let (query_schema, executor) = tokio::join!(query_schema_fut, executor_fut);

            Ok(ConnectedEngine {
                schema: builder.schema.clone(),
                query_schema: Arc::new(query_schema.unwrap()),
                executor: executor?,
                engine_protocol: builder.engine_protocol,
                native: ConnectedEngineNative {
                    config_dir: builder.native.config_dir.clone(),
                    env: builder.native.env.clone(),
                    metrics: None,
                },
            }) as Result<ConnectedEngine>
        }
        .instrument(span)
        .await?;

        *inner = Inner::Connected(engine);
        Ok(())
    }

    pub async fn query(
        &self,
        body_str: *const c_char,
        trace_str: *const c_char,
        tx_id_str: *const c_char,
    ) -> Result<String> {
        let dispatcher = self.logger.dispatcher();

        async move {
            let inner = self.inner.read().await;
            let engine = inner.as_engine()?;

            let body = get_cstr_safe(body_str).expect("Prisma engine execute body is missing");
            let tx_id = get_cstr_safe(tx_id_str);
            let trace = get_cstr_safe(trace_str).expect("Trace is needed");

            let query = RequestBody::try_from_str(&body, engine.engine_protocol())?;

            let span = tracing::info_span!("prisma:engine", user_facing = true);
            let trace_id = telemetry::helpers::set_parent_context_from_json_str(&span, &trace);

            async move {
                let handler = RequestHandler::new(engine.executor(), engine.query_schema(), engine.engine_protocol());
                let response = handler.handle(query, tx_id.map(TxId::from), trace_id).await;

                let serde_span = tracing::info_span!("prisma:engine:response_json_serialization", user_facing = true);
                Ok(serde_span.in_scope(|| serde_json::to_string(&response))?)
            }
            .instrument(span)
            .await
        }
        .with_subscriber(dispatcher)
        .await
    }

    /// Disconnect and drop the core. Can be reconnected later with `#connect`.
    pub async fn disconnect(&self, trace_str: *const c_char) -> Result<()> {
        let trace = get_cstr_safe(trace_str).expect("Trace is needed");
        let dispatcher = self.logger.dispatcher();
        async {
            let span = tracing::info_span!("prisma:engine:disconnect");
            let _ = telemetry::helpers::set_parent_context_from_json_str(&span, &trace);

            async {
                let mut inner = self.inner.write().await;
                let engine = inner.as_engine()?;

                let builder = EngineBuilder {
                    schema: engine.schema.clone(),
                    engine_protocol: engine.engine_protocol(),
                    native: EngineBuilderNative {
                        config_dir: engine.native.config_dir.clone(),
                        env: engine.native.env.clone(),
                    },
                };

                *inner = Inner::Builder(builder);

                Ok(())
            }
            .instrument(span)
            .await
        }
        .with_subscriber(dispatcher)
        .await
    }

    async unsafe fn apply_migrations(&self, migration_folder_path: *const c_char) -> Result<()> {
        if let Some(base_path) = self.base_path.as_ref() {
            env::set_current_dir(Path::new(&base_path)).expect("Could not change directory");
        }
        let migration_folder_path_str = get_cstr_safe(migration_folder_path).unwrap();
        let migration_folder_path = Path::new(&migration_folder_path_str);
        let migrations_from_filesystem = list_migration_dir(migration_folder_path)?;

        let url = self.url.clone();
        let url_without_prefix = url.strip_prefix("file:").unwrap_or(&url);
        let database_path = Path::new(url_without_prefix);

        let migrations_from_database = list_migrations(database_path).unwrap();

        let unapplied_migrations: Vec<&MigrationDirectory> = migrations_from_filesystem
            .iter()
            .filter(|fs_migration| {
                !migrations_from_database
                    .iter()
                    .filter(|db_migration: &&crate::migrations::MigrationRecord| db_migration.finished_at.is_some())
                    .any(|db_migration| fs_migration.migration_name() == db_migration.migration_name)
            })
            .collect();

        detect_failed_migrations(&migrations_from_database)?;

        let mut applied_migration_names: Vec<String> = Vec::with_capacity(unapplied_migrations.len());

        for unapplied_migration in unapplied_migrations {
            let script = unapplied_migration.read_migration_script()?;

            record_migration_started(database_path, unapplied_migration.migration_name())?;

            execute_migration_script(database_path, unapplied_migration.migration_name(), &script)?;

            applied_migration_names.push(unapplied_migration.migration_name().to_owned());
        }

        Ok(())
    }

    /// If connected, attempts to start a transaction in the core and returns its ID.
    pub async fn start_transaction(&self, input_str: *const c_char, trace_str: *const c_char) -> Result<String> {
        let input = get_cstr_safe(input_str).expect("Input string missing");
        let trace = get_cstr_safe(trace_str).expect("trace is required in transactions");
        let inner = self.inner.read().await;
        let engine = inner.as_engine()?;

        let dispatcher = self.logger.dispatcher();

        async move {
            let span = tracing::info_span!("prisma:engine:itx_runner", user_facing = true, itx_id = field::Empty);
            telemetry::helpers::set_parent_context_from_json_str(&span, &trace);

            let tx_opts: TransactionOptions = serde_json::from_str(&input)?;
            match engine
                .executor()
                .start_tx(engine.query_schema().clone(), engine.engine_protocol(), tx_opts)
                .instrument(span)
                .await
            {
                Ok(tx_id) => Ok(json!({ "id": tx_id.to_string() }).to_string()),
                Err(err) => Ok(map_known_error(err)?),
            }
        }
        .with_subscriber(dispatcher)
        .await
    }

    // If connected, attempts to commit a transaction with id `tx_id` in the core.
    pub async fn commit_transaction(&self, tx_id_str: *const c_char, _trace: *const c_char) -> Result<String> {
        let tx_id = get_cstr_safe(tx_id_str).expect("Input string missing");
        let inner = self.inner.read().await;
        let engine = inner.as_engine()?;

        let dispatcher = self.logger.dispatcher();

        async move {
            match engine.executor().commit_tx(TxId::from(tx_id)).await {
                Ok(_) => Ok("{}".to_string()),
                Err(err) => Ok(map_known_error(err)?),
            }
        }
        .with_subscriber(dispatcher)
        .await
    }

    // If connected, attempts to roll back a transaction with id `tx_id` in the core.
    pub async fn rollback_transaction(&self, tx_id_str: *const c_char, _trace: *const c_char) -> Result<String> {
        let tx_id = get_cstr_safe(tx_id_str).expect("Input string missing");
        // let trace = get_cstr_safe(trace_str).expect("trace is required in transactions");
        let inner = self.inner.read().await;
        let engine = inner.as_engine()?;

        let dispatcher = self.logger.dispatcher();

        async move {
            match engine.executor().rollback_tx(TxId::from(tx_id)).await {
                Ok(_) => Ok("{}".to_string()),
                Err(err) => Ok(map_known_error(err)?),
            }
        }
        .with_subscriber(dispatcher)
        .await
    }
}

//            _____ _____
//      /\   |  __ \_   _|
//     /  \  | |__) || |
//    / /\ \ |  ___/ | |
//   / ____ \| |    _| |_
//  /_/    \_\_|   |_____|
//
// This API is meant to be stateless. This means the box pointer to the query engine structure will be returned to the
// calling code and should be passed to subsequent calls
//
// We should be careful about not de-allocating the pointer
// when adding a new function remember to always call mem::forget

/// # Safety
/// The calling context needs to pass a valid pointer that will store the reference
/// The calling context also need to clear the pointer of the error string if it is not null
#[no_mangle]
pub unsafe extern "C" fn prisma_create(
    options: ConstructorOptions,
    qe_ptr: *mut *mut QueryEngine,
    error_string_ptr: *mut *mut c_char,
) -> c_int {
    if qe_ptr.is_null() {
        return PRISMA_MISSING_POINTER;
    }

    let res = QueryEngine::new(options);
    match res {
        Ok(v) => {
            *qe_ptr = Box::into_raw(Box::new(v));
            PRISMA_OK
        }
        Err(err) => {
            let error_string = CString::new(err.to_string()).unwrap();
            *error_string_ptr = error_string.into_raw() as *mut c_char;
            PRISMA_UNKNOWN_ERROR
        }
    }
}

/// # Safety
///
/// The calling context needs to pass a valid pointer that will store the reference to the error string
/// The calling context also need to clear the pointer of the error string if it is not null
#[no_mangle]
pub unsafe extern "C" fn prisma_connect(
    qe: *mut QueryEngine,
    trace: *const c_char,
    error_string_ptr: *mut *mut c_char,
) -> c_int {
    let query_engine: Box<QueryEngine> = Box::from_raw(qe);
    let result = RUNTIME.block_on(async { query_engine.connect(trace).await });

    match result {
        Ok(_engine) => {
            std::mem::forget(query_engine);
            *error_string_ptr = std::ptr::null_mut();
            PRISMA_OK
        }
        Err(err) => {
            let error_string = CString::new(err.to_string()).unwrap();
            *error_string_ptr = error_string.into_raw() as *mut c_char;
            std::mem::forget(query_engine);
            PRISMA_UNKNOWN_ERROR
        }
    }
}

/// # Safety
///
/// The calling context needs to pass a valid pointer that will store the reference to the error string
/// The calling context also need to clear the pointer of the error string if it is not null
#[no_mangle]
pub unsafe extern "C" fn prisma_query(
    qe: *mut QueryEngine,
    body_str: *const c_char,
    header_str: *const c_char,
    tx_id_str: *const c_char,
    error_string_ptr: *mut *mut c_char,
) -> *const c_char {
    let query_engine: Box<QueryEngine> = Box::from_raw(qe);
    let result = RUNTIME.block_on(async { query_engine.query(body_str, header_str, tx_id_str).await });
    match result {
        Ok(query_result) => {
            std::mem::forget(query_engine);
            *error_string_ptr = std::ptr::null_mut();
            CString::new(query_result).unwrap().into_raw()
        }
        Err(err) => {
            let error_string = CString::new(err.to_string()).unwrap();
            *error_string_ptr = error_string.into_raw() as *mut c_char;

            std::mem::forget(query_engine);
            null_mut()
        }
    }
}

/// # Safety
///
/// The calling context needs to pass a valid pointer that will store the reference to the error string
/// The calling context also need to clear the pointer of the error string if it is not null
#[no_mangle]
pub unsafe extern "C" fn prisma_start_transaction(
    qe: *mut QueryEngine,
    options_str: *const c_char,
    header_str: *const c_char,
) -> *const c_char {
    let query_engine: Box<QueryEngine> = Box::from_raw(qe);
    let result = RUNTIME.block_on(async { query_engine.start_transaction(options_str, header_str).await });
    match result {
        Ok(query_result) => {
            std::mem::forget(query_engine);
            CString::new(query_result).unwrap().into_raw()
        }
        Err(err) => {
            std::mem::forget(query_engine);
            CString::new(serialize_api_error(err)).unwrap().into_raw()
        }
    }
}

/// # Safety
///
/// The calling context needs to pass a valid pointer that will store the reference to the error string
#[no_mangle]
pub unsafe extern "C" fn prisma_commit_transaction(
    qe: *mut QueryEngine,
    tx_id_str: *const c_char,
    header_str: *const c_char,
) -> *const c_char {
    let query_engine: Box<QueryEngine> = Box::from_raw(qe);
    let result = RUNTIME.block_on(async { query_engine.commit_transaction(tx_id_str, header_str).await });
    std::mem::forget(query_engine);
    match result {
        Ok(query_result) => CString::new(query_result).unwrap().into_raw(),
        Err(err) => CString::new(serialize_api_error(err)).unwrap().into_raw(),
    }
}

/// # Safety
///
/// The calling context needs to pass a valid pointer that will store the reference to the error string
#[no_mangle]
pub unsafe extern "C" fn prisma_rollback_transaction(
    qe: *mut QueryEngine,
    tx_id_str: *const c_char,
    header_str: *const c_char,
) -> *const c_char {
    let query_engine: Box<QueryEngine> = Box::from_raw(qe);
    let result = RUNTIME.block_on(async { query_engine.rollback_transaction(tx_id_str, header_str).await });
    std::mem::forget(query_engine);
    match result {
        Ok(query_result) => CString::new(query_result).unwrap().into_raw(),
        Err(err) => CString::new(serialize_api_error(err)).unwrap().into_raw(),
    }
}

/// # Safety
///
/// The calling context needs to pass a valid pointer that will store the reference to the error string
#[no_mangle]
pub unsafe extern "C" fn prisma_disconnect(qe: *mut QueryEngine, header_str: *const c_char) -> c_int {
    let query_engine: Box<QueryEngine> = Box::from_raw(qe);
    let result = RUNTIME.block_on(async { query_engine.disconnect(header_str).await });
    std::mem::forget(query_engine);
    match result {
        Ok(_) => PRISMA_OK,
        Err(_err) => PRISMA_UNKNOWN_ERROR,
    }
}

/// # Safety
///
/// The calling context needs to pass a valid pointer that will store the reference to the error string
/// The calling context also need to clear the pointer of the error string if it is not null
#[no_mangle]
pub unsafe extern "C" fn prisma_apply_pending_migrations(
    qe: *mut QueryEngine,
    migration_folder_path: *const c_char,
    error_string_ptr: *mut *mut c_char,
) -> c_int {
    let query_engine: Box<QueryEngine> = Box::from_raw(qe);
    let result = RUNTIME.block_on(async { query_engine.apply_migrations(migration_folder_path).await });
    match result {
        Ok(_) => {
            std::mem::forget(query_engine);
            *error_string_ptr = std::ptr::null_mut();
            PRISMA_OK
        }
        Err(err) => {
            let error_string = CString::new(err.to_string()).unwrap();
            *error_string_ptr = error_string.into_raw() as *mut c_char;
            std::mem::forget(query_engine);
            PRISMA_UNKNOWN_ERROR
        }
    }
}

/// # Safety
///
/// Will destroy the pointer to the query engine
#[no_mangle]
pub unsafe extern "C" fn prisma_destroy(qe: *mut QueryEngine) -> c_int {
    // Once the variable goes out of scope, it will be deallocated
    let _query_engine: Box<QueryEngine> = Box::from_raw(qe);
    PRISMA_OK
}
