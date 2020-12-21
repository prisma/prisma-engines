use migration_connector::MigrationConnector;

pub struct MigrationEngine<C>
where
    C: MigrationConnector,
{
    connector: C,
}

impl<C: MigrationConnector> MigrationEngine<C> {
    pub fn new(connector: C) -> Self {
        MigrationEngine { connector }
    }

    pub fn connector(&self) -> &C {
        &self.connector
    }
}
