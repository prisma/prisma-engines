use super::{EnumDiffer, ModelDiffer};
use datamodel::ast;

/// Implements the logic to diff top-level items in a pair of [Datamodel ASTs](/datamodel/ast/struct.Datamodel.html).
pub(crate) struct TopDiffer<'a> {
    pub(crate) previous: &'a ast::Datamodel,
    pub(crate) next: &'a ast::Datamodel,
}

impl<'a> TopDiffer<'a> {
    /// Iterator over the models in `previous`.
    fn previous_models(&self) -> impl Iterator<Item = &ast::Model> {
        walk_models(self.previous)
    }

    /// Iterator over the models in `next`.
    fn next_models(&self) -> impl Iterator<Item = &ast::Model> {
        walk_models(self.next)
    }

    /// Iterator over the models present in both `previous` and `next`.
    pub(crate) fn model_pairs(&self) -> impl Iterator<Item = ModelDiffer<'_>> {
        self.previous_models().filter_map(move |previous_model| {
            self.next_models()
                .find(|next_model| models_match(previous_model, next_model))
                .map(|next_model| ModelDiffer {
                    previous: previous_model,
                    next: next_model,
                })
        })
    }

    /// Iterator over the models present in `next` but not `previous`.
    pub(crate) fn created_models(&self) -> impl Iterator<Item = &ast::Model> {
        self.next_models().filter(move |next_model| {
            self.previous_models()
                .find(|previous_model| models_match(previous_model, next_model))
                .is_none()
        })
    }

    /// Iterator over the models present in `previous` but not `next`.
    pub(crate) fn deleted_models(&self) -> impl Iterator<Item = &ast::Model> {
        self.previous_models().filter(move |previous_model| {
            self.next_models()
                .find(|next_model| models_match(previous_model, next_model))
                .is_none()
        })
    }

    /// Iterator over the enums in `previous`.
    pub(crate) fn previous_enums(&self) -> impl Iterator<Item = &ast::Enum> {
        walk_enums(self.previous)
    }

    /// Iterator over the enums in `next`.
    pub(crate) fn next_enums(&self) -> impl Iterator<Item = &ast::Enum> {
        walk_enums(self.next)
    }

    /// Iterator over the enums present in both `previous` and `next`.
    pub(crate) fn enum_pairs(&self) -> impl Iterator<Item = EnumDiffer<'_>> {
        self.previous_enums().filter_map(move |previous_enum| {
            self.next_enums()
                .find(|next_enum| enums_match(previous_enum, next_enum))
                .map(|next_enum| EnumDiffer {
                    previous: previous_enum,
                    next: next_enum,
                })
        })
    }

    /// Iterator over the enums present in `next` but not `previous`.
    pub(crate) fn created_enums(&self) -> impl Iterator<Item = &ast::Enum> {
        self.next_enums().filter(move |next_enum| {
            self.previous_enums()
                .find(|previous_enum| enums_match(previous_enum, next_enum))
                .is_none()
        })
    }

    /// Iterator over the enums present in `previous` but not `next`.
    pub(crate) fn deleted_enums(&self) -> impl Iterator<Item = &ast::Enum> {
        self.previous_enums().filter(move |previous_enum| {
            self.next_enums()
                .find(|next_enum| enums_match(previous_enum, next_enum))
                .is_none()
        })
    }
}

fn walk_enums(ast: &ast::Datamodel) -> impl Iterator<Item = &ast::Enum> {
    ast.models.iter().filter_map(ast::Top::as_enum)
}

fn enums_match(previous: &ast::Enum, next: &ast::Enum) -> bool {
    previous.name.name == next.name.name
}

fn walk_models(ast: &ast::Datamodel) -> impl Iterator<Item = &ast::Model> {
    ast.models.iter().filter_map(ast::Top::as_model)
}

fn models_match(previous: &ast::Model, next: &ast::Model) -> bool {
    previous.name.name == next.name.name
}

#[cfg(test)]
mod tests {
    use super::*;
    use datamodel::parse_to_ast;

    #[test]
    fn datamodel_differ_top_level_methods_work() {
        let previous = r#"
        model User {
            id Int @id
        }

        model Blog {
            id Int @id
            author User
        }

        enum Stays { A }

        enum ToBeDeleted { B }
        "#;
        let previous = parse_to_ast(previous).unwrap();
        let next = r#"
        model Author {
            id Int @id
            blogs Blog[]
        }

        model Blog {
            id Int @id
        }

        enum Stays { A }

        enum NewEnum { B }
        "#;
        let next = parse_to_ast(next).unwrap();

        let differ = TopDiffer {
            previous: &previous,
            next: &next,
        };

        let created_models: Vec<&str> = differ.created_models().map(|model| model.name.name.as_str()).collect();
        assert_eq!(created_models, &["Author"]);

        let deleted_models: Vec<&str> = differ.deleted_models().map(|model| model.name.name.as_str()).collect();
        assert_eq!(deleted_models, &["User"]);

        let model_pairs: Vec<(&str, &str)> = differ
            .model_pairs()
            .map(|model_differ| {
                (
                    model_differ.previous.name.name.as_str(),
                    model_differ.next.name.name.as_str(),
                )
            })
            .collect();
        assert_eq!(model_pairs, &[("Blog", "Blog")]);

        let created_enums: Vec<&str> = differ.created_enums().map(|enm| enm.name.name.as_str()).collect();
        assert_eq!(created_enums, &["NewEnum"]);

        let deleted_enums: Vec<&str> = differ.deleted_enums().map(|enm| enm.name.name.as_str()).collect();
        assert_eq!(deleted_enums, &["ToBeDeleted"]);

        let enum_pairs: Vec<(&str, &str)> = differ
            .enum_pairs()
            .map(|enum_differ| {
                (
                    enum_differ.previous.name.name.as_str(),
                    enum_differ.next.name.name.as_str(),
                )
            })
            .collect();
        assert_eq!(enum_pairs, &[("Stays", "Stays")]);
    }
}
