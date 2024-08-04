use enumflags2::BitFlags;
use futures::TryStreamExt;
use mongodb::bson::{self, doc};
use mongodb_schema_connector::MongoDbSchemaConnector;
use once_cell::sync::Lazy;
use psl::{parser_database::SourceFile, PreviewFeature};
use schema_connector::{ConnectorParams, DiffTarget, SchemaConnector};
use std::{
    collections::BTreeMap,
    fmt::Write as _,
    io::Write as _,
    sync::{atomic::AtomicUsize, Arc},
};
use tokio::sync::OnceCell;

static CONN_STR: Lazy<String> = Lazy::new(|| match std::env::var("TEST_DATABASE_URL") {
    Ok(url) => url,
    Err(_) => {
        let stderr = std::io::stderr();

        let mut sink = stderr.lock();
        sink.write_all(b"Please run with a TEST_DATABASE_URL env var pointing to a MongoDB instance.")
            .unwrap();
        sink.write_all(b"\n").unwrap();

        std::process::exit(1)
    }
});

static RT: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
});

async fn client() -> &'static mongodb::Client {
    static CLIENT: OnceCell<mongodb::Client> = OnceCell::const_new();

    CLIENT
        .get_or_init(|| async move { mongodb_client::create(CONN_STR.as_str()).await.unwrap() })
        .await
}

fn fresh_db_name() -> String {
    /// An atomic counter to get a unique identifier for each test database.
    static DATABASE_ID: AtomicUsize = AtomicUsize::new(0);
    const PREFIX: &str = "test_database_";

    let id = DATABASE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut out = String::with_capacity(PREFIX.len() + 4);
    out.push_str(PREFIX);
    out.write_fmt(format_args!("{id:04}")).unwrap();
    out
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct IndexDefinition {
    name: String,
    is_unique: bool,
    keys: bson::Document,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Debug)]
pub(crate) struct State {
    collections: BTreeMap<String, StateCollection>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub(crate) struct StateCollection {
    indexes: Vec<IndexDefinition>,
    documents: Vec<bson::Document>,
}

fn new_connector(preview_features: BitFlags<PreviewFeature>) -> (String, MongoDbSchemaConnector) {
    let db_name = fresh_db_name();
    let mut url: url::Url = CONN_STR.parse().unwrap();
    url.set_path(&db_name);
    let params = ConnectorParams {
        connection_string: url.to_string(),
        preview_features,
        shadow_database_connection_string: None,
    };
    (db_name, MongoDbSchemaConnector::new(params))
}

async fn get_state(db: &mongodb::Database) -> State {
    let collection_names = db.list_collection_names(None).await.unwrap();
    let mut state = State::default();

    for collection_name in collection_names {
        let collection: mongodb::Collection<bson::Document> = db.collection(&collection_name);
        let mut documents = Vec::new();
        let mut indexes = Vec::new();

        let mut cursor: mongodb::Cursor<bson::Document> = collection.find(None, None).await.unwrap();

        while let Some(doc) = cursor.try_next().await.unwrap() {
            documents.push(doc)
        }

        let mut cursor = collection.list_indexes(None).await.unwrap();

        while let Some(index) = cursor.try_next().await.unwrap() {
            let options = index.options.unwrap();
            indexes.push(IndexDefinition {
                keys: index.keys,
                is_unique: options.unique.unwrap_or(false),
                name: options.name.unwrap(),
            });
        }

        state
            .collections
            .insert(collection_name, StateCollection { indexes, documents });
    }

    state
}

async fn apply_state(db: &mongodb::Database, state: State) {
    for (collection_name, StateCollection { indexes, documents }) in state.collections {
        let collection: mongodb::Collection<bson::Document> = db.collection(&collection_name);

        if !indexes.is_empty() {
            let indexes = indexes.into_iter().map(|index| {
                let mut model = mongodb::IndexModel::default();
                let mut options = mongodb::options::IndexOptions::default();
                options.name = Some(index.name);
                options.unique = Some(index.is_unique);
                model.options = Some(options);
                model.keys = index.keys;
                model
            });

            collection.create_indexes(indexes, None).await.unwrap();
        }

        if !documents.is_empty() {
            collection.insert_many(documents, None).await.unwrap();
        }
    }
}

const SCENARIOS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/migrations/scenarios");
static UPDATE_EXPECT: Lazy<bool> = Lazy::new(|| std::env::var("UPDATE_EXPECT").is_ok());

pub(crate) fn test_scenario(scenario_name: &str) {
    let mut path = String::with_capacity(SCENARIOS_PATH.len() + 12);

    let schema = {
        write!(path, "{SCENARIOS_PATH}/{scenario_name}/schema.prisma").unwrap();
        std::fs::read_to_string(&path).unwrap()
    };

    let state: State = {
        path.clear();
        write!(path, "{SCENARIOS_PATH}/{scenario_name}/state.json").unwrap();
        let file = std::fs::File::open(&path).unwrap();
        let collections: BTreeMap<String, _> = serde_json::from_reader(&file).unwrap();
        State { collections }
    };

    let mut expected_result = {
        path.clear();
        write!(path, "{SCENARIOS_PATH}/{scenario_name}/result").unwrap();
        std::fs::read_to_string(&path).unwrap()
    };

    if cfg!(target_os = "windows") {
        expected_result.retain(|c| c != '\r');
    }

    RT.block_on(async move {
        let schema = SourceFile::new_allocated(Arc::from(schema.into_boxed_str()));
        let parsed_schema = psl::parse_schema(schema.clone()).unwrap();
        let (db_name, mut connector) = new_connector(parsed_schema.configuration.preview_features());
        let client = client().await;
        let db = client.database(&db_name);
        db.drop(None).await.unwrap();
        apply_state(&db, state).await;

        let from = connector
            .database_schema_from_diff_target(DiffTarget::Database, None, None)
            .await
            .unwrap();
        let to = connector
            .database_schema_from_diff_target(
                DiffTarget::Datamodel(vec![("schema.prisma".to_string(), schema.clone())]),
                None,
                None,
            )
            .await
            .unwrap();
        let migration = connector.diff(from, to);

        connector.apply_migration(&migration).await.unwrap();

        let state = get_state(&db).await;

        let mut rendered_migration = connector.migration_summary(&migration);
        rendered_migration.push_str("\n------\n\n");
        rendered_migration.push_str(&serde_json::to_string_pretty(&state).unwrap());
        rendered_migration.push('\n');

        if *UPDATE_EXPECT {
            let mut file = std::fs::File::create(&path).unwrap(); // truncate
            write!(file, "{rendered_migration}").unwrap();
        } else if expected_result != rendered_migration {
            let chunks = dissimilar::diff(&expected_result, &rendered_migration);
            panic!(
                r#"
Snapshot comparison failed. Run the test again with UPDATE_EXPECT=1 in the environment to update the snapshot.

===== EXPECTED ====
{}
====== FOUND ======
{}
======= DIFF ======
{}
"#,
                expected_result,
                rendered_migration,
                format_chunks(chunks),
            );
        }

        // Check that the migration is idempotent.
        let from = connector
            .database_schema_from_diff_target(DiffTarget::Database, None, None)
            .await
            .unwrap();
        let to = connector
            .database_schema_from_diff_target(
                DiffTarget::Datamodel(vec![("schema.prisma".to_string(), schema.clone())]),
                None,
                None,
            )
            .await
            .unwrap();
        let migration = connector.diff(from, to);

        assert!(
            connector.migration_is_empty(&migration),
            "Expected an empty migration when applying the same schema, got:\n{}",
            connector.migration_summary(&migration)
        );
    })
}

fn format_chunks(chunks: Vec<dissimilar::Chunk>) -> String {
    let mut buf = String::new();
    for chunk in chunks {
        let formatted = match chunk {
            dissimilar::Chunk::Equal(text) => text.into(),
            dissimilar::Chunk::Delete(text) => format!("\x1b[41m{text}\x1b[0m"),
            dissimilar::Chunk::Insert(text) => format!("\x1b[42m{text}\x1b[0m"),
        };
        buf.push_str(&formatted);
    }
    buf
}
