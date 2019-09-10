use crate::error::Error;
use mysql as my;

impl From<my::error::Error> for Error {
    fn from(e: my::error::Error) -> Error {
        use my::error::MySqlError;

        match e {
            my::error::Error::MySqlError(MySqlError {
                ref message,
                code,
                ..
            }) if code == 1062 => {
                let splitted: Vec<&str> = message.split_whitespace().collect();
                let splitted: Vec<&str> = splitted.last().map(|s| s.split('\'').collect()).unwrap();
                let splitted: Vec<&str> = splitted[1].split('_').collect();

                let field_name: String = splitted[0].into();

                Error::UniqueConstraintViolation { field_name }
            }
            my::error::Error::MySqlError(MySqlError {
                ref message,
                code,
                ..
            }) if code == 1263 => {
                let splitted: Vec<&str> = message.split_whitespace().collect();
                let splitted: Vec<&str> = splitted.last().map(|s| s.split('\'').collect()).unwrap();
                let splitted: Vec<&str> = splitted[1].split('_').collect();

                let field_name: String = splitted[0].into();

                Error::NullConstraintViolation { field_name }
            }
            my::error::Error::MySqlError(MySqlError {
                ref message,
                code,
                ..
            }) if code == 1049 => {
                let splitted: Vec<&str> = dbg!(message.split_whitespace().collect());
                let splitted: Vec<&str> = dbg!(splitted.last().map(|s| s.split('\'').collect()).unwrap());
                let db_name: String = dbg!(splitted[1]).into();

                Error::DatabaseDoesNotExist(db_name)
            }
            e => Error::QueryError(e.into()),
        }
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(e: std::string::FromUtf8Error) -> Error {
        Error::QueryError(e.into())
    }
}
