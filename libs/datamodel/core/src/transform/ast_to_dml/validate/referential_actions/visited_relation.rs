use std::{fmt, rc::Rc};

use itertools::Itertools;

/// A linked list structure for visited relation paths.
#[derive(Debug, Clone)]
pub(crate) struct VisitedRelation<'a> {
    previous: Option<Rc<VisitedRelation<'a>>>,
    model_name: &'a str,
    field_name: Option<&'a str>,
}

impl<'a> VisitedRelation<'a> {
    /// Create a new root node, starting a new relation path.
    pub(crate) fn root(model_name: &'a str, field_name: &'a str) -> Self {
        Self {
            previous: None,
            model_name,
            field_name: Some(field_name),
        }
    }

    /// Links the final model
    pub(crate) fn link_model(self: &Rc<Self>, model_name: &'a str) -> Self {
        Self {
            previous: Some(self.clone()),
            model_name,
            field_name: None,
        }
    }

    /// Links a relation to the current path.
    pub(crate) fn link_next(self: &Rc<Self>, model_name: &'a str, field_name: &'a str) -> Self {
        Self {
            previous: Some(self.clone()),
            model_name,
            field_name: Some(field_name),
        }
    }

    /// Converts the list into an iterator.
    pub(crate) fn iter(&self) -> RelationIter<'a> {
        let mut traversed_models = vec![(self.model_name, self.field_name)];
        let mut this = self;

        while let Some(next) = this.previous.as_ref() {
            traversed_models.push((next.model_name, next.field_name));
            this = next;
        }

        RelationIter { traversed_models }
    }
}

impl<'a> fmt::Display for VisitedRelation<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut traversed = self.iter().map(|(model_name, field_name)| {
            if let Some(field_name) = field_name {
                format!("{}.{}", model_name, field_name)
            } else {
                model_name.to_string()
            }
        });

        write!(f, "{}", traversed.join(" → "))
    }
}

pub(crate) struct RelationIter<'a> {
    traversed_models: Vec<(&'a str, Option<&'a str>)>,
}

impl<'a> Iterator for RelationIter<'a> {
    type Item = (&'a str, Option<&'a str>);

    fn next(&mut self) -> Option<Self::Item> {
        self.traversed_models.pop()
    }
}
