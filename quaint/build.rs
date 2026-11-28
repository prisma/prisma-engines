use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        native: {
            any(
                feature = "mssql-native",
                feature = "mysql-native",
                feature = "kingbase-mysql-native",
                feature = "kingbase-oracle-native",
                feature = "postgresql-native",
                feature = "sqlite-native"
            )
        }
    }
}
