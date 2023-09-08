use schema_core::schema_connector::MigrationRecord;

pub trait MigrationsAssertions: Sized {
    fn assert_applied_steps_count(self, count: u32) -> Self;
    fn assert_checksum(self, expected: &str) -> Self;
    fn assert_failed(self) -> Self;
    fn assert_logs(self, expected: &str) -> Self;
    fn assert_migration_name(self, expected: &str) -> Self;
    fn assert_success(self) -> Self;
}

impl MigrationsAssertions for MigrationRecord {
    fn assert_checksum(self, expected: &str) -> Self {
        assert_eq!(self.checksum, expected);
        self
    }

    fn assert_migration_name(self, expected: &str) -> Self {
        assert_eq!(&self.migration_name[15..], expected);
        self
    }

    fn assert_logs(self, expected: &str) -> Self {
        assert_eq!(self.logs.as_deref(), Some(expected));
        self
    }

    fn assert_applied_steps_count(self, count: u32) -> Self {
        assert_eq!(self.applied_steps_count, count);
        self
    }

    fn assert_success(self) -> Self {
        assert!(self.finished_at.is_some());
        self
    }

    fn assert_failed(self) -> Self {
        assert!(self.finished_at.is_none() && self.rolled_back_at.is_none());
        self
    }
}
