mod git_repo;

use crate::{
    misc_helpers::*,
    test_api::{Apply, Infer},
};
use anyhow::Context;
use futures::{future::Ready, FutureExt, TryFutureExt};
use git_repo::GitRepo;
use migration_connector::{Migration, MigrationConnector};
use migration_core::{api::MigrationApi, commands::MigrationStepsResultOutput};
use std::{
    io::{Read as _, Write as _},
    path::{Path, PathBuf},
};
pub use test_macros::test_each_connector;
use test_setup::*;
use tracing_futures::Instrument;

type BoxFut<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + 'a>>;

/// The root directory for the test. Be careful changing this, we delete this directory as part of
/// test setup.
fn root_dir(test_name: &str, connector_name: &str) -> PathBuf {
    let root = server_root();

    Path::new(&root).join("db").join(test_name).join(connector_name)
}

pub struct TestApi {
    #[allow(unused)]
    /// Unique identifier for a connector (e.g. postgres12)
    connector_name: &'static str,
    db_name_root: &'static str,
    provider: &'static str,
    root_dir: PathBuf,
    url_factory: fn(&str) -> String,
}

impl TestApi {
    fn new(
        db_name: &'static str,
        url_factory: fn(&str) -> String,
        connector_name: &'static str,
        provider: &'static str,
    ) -> Self {
        // /!\ /!\ /!\ DANGER ZONE /!\ /!\ /!\
        let root_dir = root_dir(db_name, connector_name);
        std::fs::remove_dir_all(&root_dir).ok();
        std::fs::create_dir_all(&root_dir)
            .with_context(|| format!("Creating root directory at {:?}.", &root_dir))
            .unwrap();
        // /!\ /!\ /!\ DANGER ZONE /!\ /!\ /!\

        TestApi {
            root_dir,
            db_name_root: db_name,
            provider,
            url_factory,
            connector_name,
        }
    }

    pub async fn new_user<'a, 'b>(&'a self, name: &'static str, dm: &'b str) -> anyhow::Result<User<'a>>
    where
        'a: 'b,
    {
        Ok(User::new(self, name, dm).await?)
    }

    fn db_url(&self, db_name: &str) -> String {
        (self.url_factory)(db_name)
    }

    pub async fn user_cloned_from<'a, 'b>(
        &'a self,
        user: &'a User<'b>,
        name: &'static str,
    ) -> anyhow::Result<User<'b>> {
        User::clone_from(user, name).await
    }
}

pub struct User<'a> {
    name: &'static str,
    api: &'a TestApi,
    migrations_folder: PathBuf,
    schema_path: PathBuf,
    git_repo: GitRepo,
    engine: MigrationApi<sql_migration_connector::SqlMigrationConnector, sql_migration_connector::SqlMigration>,
}

impl<'a> User<'a> {
    fn clone_from<'b>(other: &'b User<'a>, name: &'static str) -> BoxFut<'b, anyhow::Result<User<'a>>> {
        let api = other.api;

        tracing::debug!("cloning user {} from {}", name, other.name);

        let user_dir = api.root_dir.join(name);
        let migrations_folder = user_dir.join("migrations");
        let schema_path = user_dir.join("schema.prisma");
        let repo = git2::Repository::clone(other.git_repo.root_dir().as_os_str().to_str().unwrap(), &user_dir)
            .expect("cloning");

        let git_repo = GitRepo {
            name,
            root_dir: user_dir,
            repo,
        };

        let db_name = format!("{}_{}", other.api.db_name_root, name);
        let url = other.api.db_url(&db_name);

        user_engine(db_name, url, api.provider)
            .map_ok(move |engine| User {
                engine,
                name,
                api,
                migrations_folder,
                schema_path,
                git_repo,
            })
            .boxed_local()
    }

    async fn new<'b>(api: &'a TestApi, name: &'static str, dm: &'b str) -> anyhow::Result<User<'a>>
    where
        'a: 'b,
    {
        let user_dir: PathBuf = api.root_dir.join(name);
        let migrations_folder = user_dir.join("migrations");
        let schema_path = user_dir.join("schema.prisma");

        std::fs::create_dir(&user_dir).with_context(|| format!("Creating user dir for {} at {:?}", name, user_dir))?;
        std::fs::create_dir(&migrations_folder)?;
        std::fs::File::create(migrations_folder.join(".keep"))?;

        let git_repo = GitRepo {
            name,
            repo: git2::Repository::init(&user_dir)?,
            root_dir: user_dir,
        };

        let mut schema_file = std::fs::File::create(&schema_path)
            .with_context(|| format!("While creating the schema.prisma at {:?}", &schema_path))?;
        schema_file.write_all(dm.as_bytes())?;

        let db_name = format!("{}_{}", api.db_name_root, name);
        let url = api.db_url(&db_name);

        let engine = user_engine(db_name, url, api.provider).await?;
        let user = User {
            engine,
            name,
            api,
            migrations_folder,
            schema_path,
            git_repo,
        };
        user.git_repo.commit("Initial commit")?;

        Ok(user)
    }

    fn connector(&self) -> &sql_migration_connector::SqlMigrationConnector {
        self.engine.connector()
    }

    pub fn assert_cannot_merge(&self, other: &User<'_>) -> anyhow::Result<()> {
        let (_last_commit, mut index) = self.git_repo.prepare_merge(&other.git_repo)?;

        match index.write_tree_to(&self.git_repo.repo) {
            Ok(_oid) => anyhow::bail!(
                "Assertion failed: successfully merged {} into {}",
                other.name,
                self.name
            ),
            Err(err) if err.code() == git2::ErrorCode::Unmerged => Ok(()),
            Err(err) => Err(err.into()),
        }
    }

    pub fn schema_string(&self) -> anyhow::Result<String> {
        let mut file = std::fs::File::open(&self.schema_path)
            .with_context(|| format!("opening schema file at {:?} for user {}", &self.schema_path, &self.name))?;

        let mut out = String::new();
        file.read_to_string(&mut out)?;

        Ok(out)
    }

    pub fn write_schema(&self, schema: &str) -> anyhow::Result<()> {
        let mut file = std::fs::File::create(&self.schema_path).context("opening schema for writes")?;
        file.write_all(schema.as_bytes()).context("writing schema")?;

        self.git_repo.commit("Changed prisma schema")?;

        Ok(())
    }

    pub fn migrations_in_folder(&self) -> anyhow::Result<Vec<String>> {
        let mut entries: Vec<String> = std::fs::read_dir(&self.migrations_folder)?
            .map(|entry| {
                entry
                    .map(|entry| entry.path().file_stem().unwrap().to_str().unwrap().to_owned())
                    .map_err(anyhow::Error::from)
            })
            .filter(|entry| entry.as_ref().map(|entry| entry != ".keep").unwrap_or(false))
            .collect::<Result<_, _>>()?;

        entries.sort();

        Ok(entries)
    }

    pub fn load_migration_files(&self) -> anyhow::Result<Vec<(String, MigrationStepsResultOutput)>> {
        let entries = std::fs::read_dir(&self.migrations_folder)?;
        let mut entries: Vec<PathBuf> = entries
            .map(|entry| -> Result<_, _> { Ok(entry?.path()) })
            .filter(|entry| {
                entry
                    .as_ref()
                    .ok()
                    .and_then(|entry| entry.file_name())
                    .map(|entry| entry != ".keep")
                    .unwrap_or(false)
            })
            .collect::<Result<Vec<PathBuf>, anyhow::Error>>()?;

        // Important, otherwise we can't determine which migration is next to apply.
        entries.sort();

        let mut migrations = Vec::with_capacity(entries.len());

        for entry in entries {
            let mut file = std::fs::File::open(&entry)?;
            let migration = serde_json::from_reader(&mut file)
                .with_context(|| format!("Error reading migration from file {:?}", entry))?;
            let migration_name = entry.file_stem().unwrap().to_str().unwrap().to_owned();

            migrations.push((migration_name, migration));
        }

        Ok(migrations)
    }

    pub fn persist_migration<'b>(
        &'b self,
        infer_output: &'b MigrationStepsResultOutput,
        migration_name: &'static str,
    ) -> anyhow::Result<()>
    where
        'a: 'b,
    {
        let target_file_path = self.migrations_folder.join(format!("{}.json", migration_name));
        let mut file = std::fs::File::create(&target_file_path)?;
        serde_json::to_writer_pretty(&mut file, &infer_output)?;

        Ok(())
    }

    pub async fn list_migrations(&self) -> anyhow::Result<Vec<Migration>> {
        let connector = self.connector();
        let persistence = connector.migration_persistence();

        Ok(persistence.load_all().await?)
    }

    async fn unapplied_migrations(&self) -> anyhow::Result<impl Iterator<Item = (String, MigrationStepsResultOutput)>> {
        // List migrations
        let migrations_from_db = self.list_migrations().await?;
        // Load local migrations and filter out the ones that need to be applied
        let migrations_from_files = self.load_migration_files()?;

        let unapplied_migrations = migrations_from_files.into_iter().filter(move |(name, _mig)| {
            migrations_from_db
                .iter()
                .filter(|db_migration| db_migration.status.is_success())
                .find(|db_migration| db_migration.name.as_str() == name.as_str())
                .is_none()
        });

        Ok(unapplied_migrations)
    }

    /// Simulate `prisma2 migrate save`
    pub fn save(&self, migration_name: &'static str) -> Save<'_> {
        Save::new(self, migration_name)
    }

    /// Simulate `prisma2 migrate up`
    pub fn up(&self) -> Up<'_> {
        Up::new(self)
    }

    pub fn merge_from(&self, other: &User<'_>) -> anyhow::Result<()> {
        self.git_repo.merge_from(&other.git_repo)
    }
}

pub struct Save<'a> {
    user: &'a User<'a>,
    migration_name: &'static str,
}

impl<'a> Save<'a> {
    fn new(user: &'a User, migration_name: &'static str) -> Self {
        Save { user, migration_name }
    }

    pub fn execute(&self) -> BoxFut<'_, anyhow::Result<()>> {
        self.execute_inner()
            .map_ok(drop)
            .instrument(tracing::info_span!("MigrateSave", user = self.user.name))
            .boxed_local()
    }

    pub fn assert_is_up_to_date(&self) -> BoxFut<'_, anyhow::Result<()>> {
        self.execute_inner()
            .instrument(tracing::info_span!("MigrateSave", user = self.user.name))
            .and_then(|result| {
                futures::future::lazy(move |_| {
                    anyhow::ensure!(
                        result.datamodel_steps.is_empty(),
                        "Assertion failed. Datamodel steps ain't empty.\n{:?}",
                        result.datamodel_steps
                    );

                    Ok(())
                })
            })
            .boxed_local()
    }

    async fn execute_inner(&self) -> anyhow::Result<MigrationStepsResultOutput> {
        let assume_to_be_applied = self
            .user
            .unapplied_migrations()
            .await?
            .flat_map(|(_name, mig)| mig.datamodel_steps.into_iter())
            .collect::<Vec<_>>();
        let schema = self.user.schema_string()?;
        let infer_result = Infer::new(&self.user.engine, &schema)
            .migration_id(Some(self.migration_name))
            .assume_to_be_applied(Some(assume_to_be_applied))
            .send()
            .await?;

        self.user.persist_migration(&infer_result, self.migration_name)?;

        self.user.git_repo.commit("ran migrate save")?;

        Ok(infer_result)
    }
}

pub struct Up<'a> {
    user: &'a User<'a>,
}

impl<'a> Up<'a> {
    fn new(user: &'a User) -> Self {
        Up { user }
    }

    // Reference: https://github.com/prisma/migrate/blob/811ff27f231f84b8a51ca8fdb8943b8d87f9a4a1/src/Lift.ts#L792
    pub fn execute(&self) -> BoxFut<'_, anyhow::Result<()>> {
        let fut = async move {
            let engine = &self.user.engine;

            let mut unapplied_migrations = self.user.unapplied_migrations().await?;

            if let Some((name, migration)) = unapplied_migrations.next() {
                dbg!(&name);
                // Apply the first migration to be applied
                Apply::new(engine)
                    .migration_id(Some(name.clone()))
                    .steps(Some(migration.datamodel_steps.clone()))
                    .send()
                    .await?;

                Ok(())
            } else {
                anyhow::bail!("Found no migration to apply for user {}", &self.user.name);
            }
        };

        fut.instrument(tracing::info_span!("MigrateUp", user = self.user.name))
            .boxed_local()
    }
}

pub type TestResult = Result<(), anyhow::Error>;

fn user_engine<'b>(
    db_name: String,
    url: String,
    provider: &'static str,
) -> BoxFut<
    'b,
    anyhow::Result<MigrationApi<sql_migration_connector::SqlMigrationConnector, sql_migration_connector::SqlMigration>>,
> {
    let fut = async move {
        let connector = match provider {
            "mysql" => mysql_migration_connector(&url).await,
            "postgres" => postgres_migration_connector(&url).await,
            "sqlite" => sqlite_migration_connector(&db_name).await,
            other => panic!("unknown provider: {:?}", other),
        };

        let api = test_api(connector).await;

        Ok(api)
    };

    fut.boxed_local()
}

pub fn mysql_8_test_api(db_name: &'static str) -> Ready<TestApi> {
    futures::future::ready(TestApi::new(db_name, mysql_8_url, "mysql8", "mysql"))
}

pub fn mysql_5_6_test_api(db_name: &'static str) -> Ready<TestApi> {
    futures::future::ready(TestApi::new(db_name, mysql_5_6_url, "mysql_5_6", "mysql"))
}

pub fn mysql_test_api(db_name: &'static str) -> Ready<TestApi> {
    futures::future::ready(TestApi::new(db_name, mysql_url, "mysql", "mysql"))
}

pub fn postgres_test_api(db_name: &'static str) -> Ready<TestApi> {
    futures::future::ready(TestApi::new(db_name, postgres_10_url, "postgresql", "postgres"))
}

pub fn postgres9_test_api(db_name: &'static str) -> Ready<TestApi> {
    futures::future::ready(TestApi::new(db_name, postgres_9_url, "postgresql9", "postgres"))
}

pub fn postgres11_test_api(db_name: &'static str) -> Ready<TestApi> {
    futures::future::ready(TestApi::new(db_name, postgres_11_url, "postgresql11", "postgres"))
}

pub fn postgres12_test_api(db_name: &'static str) -> Ready<TestApi> {
    futures::future::ready(TestApi::new(db_name, postgres_12_url, "postgresql12", "postgres"))
}

pub fn sqlite_test_api(db_name: &'static str) -> Ready<TestApi> {
    futures::future::ready(TestApi::new(db_name, sqlite_test_file, "sqlite", "sqlite"))
}

pub fn mysql_mariadb_test_api(db_name: &'static str) -> Ready<TestApi> {
    futures::future::ready(TestApi::new(db_name, mariadb_url, "mysql_mariadb", "mysql"))
}
