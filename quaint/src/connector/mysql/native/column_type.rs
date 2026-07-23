use crate::connector::ColumnType;
use mysql_async::Column as MysqlColumn;

impl From<&MysqlColumn> for ColumnType {
    fn from(value: &MysqlColumn) -> Self {
        ColumnType::from_type_identifier(value)
    }
}
