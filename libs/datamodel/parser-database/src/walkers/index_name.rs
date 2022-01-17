use super::ScalarFieldWalker;

/// Represents the name of an index

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum IndexName<'ast> {
    /// Explicit index name defined in the datamodel
    Explicit(&'ast str),
    /// Generated index name based on the fields of the index.
    /// Only used on compound ids
    Generated(Option<String>),
}

impl<'ast> PartialEq<&str> for IndexName<'ast> {
    fn eq(&self, other: &&str) -> bool {
        match (self, other) {
            (Self::Explicit(index_name), other) => index_name == other,
            (Self::Generated(Some(index_name)), other) => index_name == other,
            (Self::Generated(None), _) => false,
        }
    }
}

impl<'ast> IndexName<'ast> {
    /// Instantiate a new `Explicit` variant.
    pub fn explicit(index_name: &'ast str) -> Self {
        Self::Explicit(index_name)
    }

    /// Instantiate a new `Generated` variant.
    pub fn generated<'db>(fields: &[ScalarFieldWalker<'ast, 'db>]) -> Self {
        if fields.len() < 2 {
            Self::Generated(None)
        } else {
            Self::Generated(Some(compute_generated_index_name(fields)))
        }
    }

    /// Try to interpret the index name as `Explicit`.
    pub fn as_explicit(&self) -> Option<&'ast str> {
        if let Self::Explicit(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Try to interpret the index name as `Generated`.
    pub fn as_generated(&self) -> Option<Option<&String>> {
        if let Self::Generated(v) = self {
            Some(v.as_ref())
        } else {
            None
        }
    }
}

/// Computes a generated index name. eg:
/// @@unique([a, b]) -> "a_b"
pub(crate) fn compute_generated_index_name(fields: &[ScalarFieldWalker<'_, '_>]) -> String {
    let parts: Vec<_> = fields.iter().map(|sf| sf.name()).collect();

    parts.join("_")
}
