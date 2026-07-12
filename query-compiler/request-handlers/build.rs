use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        native: {
            any(
                feature = "mongodb",
                feature = "mssql-native",
                feature = "mysql-native",
                feature = "postgresql-native",
                feature = "sqlite-native",
                feature = "cockroachdb-native"
            )
        }
    }
}
