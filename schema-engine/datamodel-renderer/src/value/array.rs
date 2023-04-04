use std::fmt;

/// An array of values.
#[derive(Debug, Default)]
pub struct Array<T: fmt::Display> {
    inner: Vec<T>,
}

impl<T: fmt::Display> From<Vec<T>> for Array<T> {
    fn from(inner: Vec<T>) -> Self {
        Self { inner }
    }
}

impl<T: fmt::Display> Array<T> {
    /// Returns `true` if the array contains no elements.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the length of the array.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Add a new value to the end of the array.
    pub fn push(&mut self, val: impl Into<T>) {
        self.inner.push(val.into());
    }

    /// Create a new array.
    pub const fn new() -> Self {
        Self { inner: Vec::new() }
    }
}

impl<T: fmt::Display> fmt::Display for Array<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;

        for (i, val) in self.inner.iter().enumerate() {
            val.fmt(f)?;

            if i < self.inner.len() - 1 {
                f.write_str(", ")?;
            }
        }

        f.write_str("]")?;

        Ok(())
    }
}
