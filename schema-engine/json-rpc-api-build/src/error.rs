use std::{backtrace::Backtrace, error::Error as StdError, fmt::Debug};

pub type CrateResult = Result<(), Error>;

pub struct Error {
    source: Option<Box<dyn StdError>>,
    bt: Backtrace,
    message: Option<String>,
}

impl<T> From<T> for Error
where
    T: std::error::Error + 'static,
{
    fn from(src: T) -> Self {
        Error {
            message: Some(src.to_string()),
            source: Some(Box::new(src)),
            bt: Backtrace::force_capture(),
        }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut src: Option<&dyn StdError> = self.source.as_deref();
        let mut indentation_levels = 0;

        if let Some(message) = &self.message {
            f.write_str(message)?;
        }

        while let Some(source) = src {
            f.write_str("\n")?;

            for _ in 0..=indentation_levels {
                f.write_str("  ")?;
            }

            f.write_fmt(format_args!("Caused by: {source}\n"))?;

            indentation_levels += 1;
            src = source.source();
        }

        f.write_fmt(format_args!("{:?}\n", self.bt))
    }
}
