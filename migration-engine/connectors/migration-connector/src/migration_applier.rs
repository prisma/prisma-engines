use crate::*;

/// Apply and unapply migrations on the connector's database.
#[async_trait::async_trait]
pub trait MigrationApplier<T>
where
    T: Send + Sync,
{
    async fn apply(&self, migration: &Migration, database_migration: &T) -> ConnectorResult<()>;

    async fn unapply(&self, migration: &Migration, database_migration: &T) -> ConnectorResult<()>;
}

pub struct MigrationApplierImpl<'a, T>
where
    T: Send + Sync + 'static,
{
    pub migration_persistence: Box<dyn MigrationPersistence + 'a>,
    pub step_applier: Box<dyn DatabaseMigrationStepApplier<T> + 'a>,
}

#[async_trait::async_trait]
impl<'a, T> MigrationApplier<T> for MigrationApplierImpl<'a, T>
where
    T: Send + Sync + 'static,
{
    async fn apply(&self, migration: &Migration, database_migration: &T) -> ConnectorResult<()> {
        assert_eq!(migration.status, MigrationStatus::Pending); // what other states are valid here?
        let mut migration_updates = migration.update_params();
        migration_updates.status = MigrationStatus::MigrationInProgress;
        self.migration_persistence.update(&migration_updates).await?;

        let apply_result = self.go_forward(&mut migration_updates, database_migration).await;

        match apply_result {
            Ok(()) => {
                migration_updates.mark_as_finished();
                self.migration_persistence.update(&migration_updates).await?;
                Ok(())
            }
            Err(err) => {
                migration_updates.status = MigrationStatus::MigrationFailure;
                migration_updates.errors = vec![format!("{:?}", err)];
                self.migration_persistence.update(&migration_updates).await?;
                Err(err)
            }
        }
    }

    async fn unapply(&self, migration: &Migration, database_migration: &T) -> ConnectorResult<()> {
        assert_eq!(migration.status, MigrationStatus::MigrationSuccess); // what other states are valid here?
        let mut migration_updates = migration.update_params();
        migration_updates.status = MigrationStatus::RollingBack;
        self.migration_persistence.update(&migration_updates).await?;

        let unapply_result = self.go_backward(&mut migration_updates, database_migration).await;

        match unapply_result {
            Ok(()) => {
                migration_updates.status = MigrationStatus::RollbackSuccess;
                self.migration_persistence.update(&migration_updates).await?;
                Ok(())
            }
            Err(err) => {
                migration_updates.status = MigrationStatus::RollbackFailure;
                migration_updates.errors = vec![format!("{:?}", err)];
                self.migration_persistence.update(&migration_updates).await?;
                Err(err)
            }
        }
    }
}

impl<'a, T> MigrationApplierImpl<'a, T>
where
    T: Send + Sync,
{
    async fn go_forward(
        &self,
        migration_updates: &mut MigrationUpdateParams,
        database_migration: &T,
    ) -> ConnectorResult<()> {
        let mut step = 0;
        while self.step_applier.apply_step(&database_migration, step).await? {
            step += 1;
            migration_updates.applied += 1;
            self.migration_persistence.update(&migration_updates).await?;
        }
        Ok(())
    }

    async fn go_backward(
        &self,
        migration_updates: &mut MigrationUpdateParams,
        database_migration: &T,
    ) -> ConnectorResult<()> {
        let mut step = 0;
        while self.step_applier.apply_step(&database_migration, step).await? {
            step += 1;
            migration_updates.rolled_back += 1;
            self.migration_persistence.update(&migration_updates).await?;
        }
        Ok(())
    }
}
