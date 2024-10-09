//! Namespaces are used in conjunction with the MultiSchema preview feature.

/// A nonempty set of namespaces.
///
/// It is assumed that the namespaces are unique.
/// It is often passed around an Option<Namespaces> for when
/// the namespaces cannot be inferred, or when the MultiSchema preview
/// feature is not enabled.
#[derive(Clone, Debug)]
pub struct Namespaces(String, Vec<String>);

impl Namespaces {
    /// Ensures the namespaces are unique.
    pub fn from_vec(v: &mut Vec<String>) -> Option<Self> {
        v.sort();
        v.dedup();
        v.pop().map(|i| Namespaces(i, v.to_vec()))
    }

    /// Unwraps the optional namespace list using the provided namespace
    /// as a default value.
    pub fn to_vec(o: Option<Self>, default_namespace: String) -> Vec<String> {
        match o {
            Some(Namespaces(s, mut vec)) => {
                vec.push(s);
                vec
            }
            None => vec![default_namespace],
        }
    }
}

impl IntoIterator for Namespaces {
    type Item = String;
    type IntoIter = std::iter::Chain<std::iter::Once<String>, <Vec<String> as IntoIterator>::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self.0).chain(self.1)
    }
}
