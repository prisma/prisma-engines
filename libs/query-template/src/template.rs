use crate::fragment::Fragment;
use crate::placeholder::PlaceholderFormat;
use std::fmt;
use std::fmt::Debug;

#[derive(Debug)]
pub struct QueryTemplate<P> {
    pub fragments: Vec<Fragment>,
    pub parameters: Vec<P>,
    pub placeholder_format: PlaceholderFormat,
}

impl<P> QueryTemplate<P> {
    pub fn new(placeholder_format: PlaceholderFormat) -> Self {
        QueryTemplate {
            fragments: Vec::new(),
            parameters: Vec::new(),
            placeholder_format,
        }
    }

    /// Formats SQL for use in legacy Query Engine.
    /// It does not support ParameterTuple fragments.
    pub fn to_sql(&self) -> Result<String, fmt::Error> {
        let mut sql = String::new();
        let mut placeholder_number = 1;
        for fragment in &self.fragments {
            match fragment {
                Fragment::StringChunk { chunk } => sql.push_str(chunk),
                Fragment::Parameter => self.placeholder_format.write(&mut sql, &mut placeholder_number)?,
                Fragment::ParameterTuple { .. } | Fragment::ParameterTupleList { .. } => return Err(fmt::Error), // Unsupported in Query Engine
            };
        }
        Ok(sql)
    }
}

impl<P> fmt::Display for QueryTemplate<P> {
    /// Should only be used for debugging and unit testing.
    /// Row parameters are shown in square brackets, which is not valid SQL.
    /// Row parameters always have a single placeholder, which
    /// does not attempt to match any actual array parameter.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut placeholder_number = 1;
        for fragment in &self.fragments {
            match fragment {
                Fragment::StringChunk { chunk } => write!(f, "{chunk}")?,
                Fragment::Parameter => self.placeholder_format.write(f, &mut placeholder_number)?,
                Fragment::ParameterTuple { .. } => {
                    f.write_str("[")?;
                    self.placeholder_format.write(f, &mut placeholder_number)?;
                    f.write_str("]")?;
                }
                Fragment::ParameterTupleList { .. } => {
                    f.write_str("[(")?;
                    self.placeholder_format.write(f, &mut placeholder_number)?;
                    f.write_str(")]")?;
                }
            }
        }
        Ok(())
    }
}
