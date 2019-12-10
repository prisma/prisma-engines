use datamodel::ast;
use migration_connector::steps::MigrationExpression;

#[derive(Debug)]
pub(crate) struct SourceArgumentsDiffer<'a> {
    pub(crate) previous: &'a ast::SourceConfig,
    pub(crate) next: &'a ast::SourceConfig,
}

impl<'a> SourceArgumentsDiffer<'a> {
    pub(crate) fn deleted_arguments(&self) -> impl Iterator<Item = &ast::Argument> {
        self.previous_arguments().filter(move |previous_argument| {
            self.next_arguments()
                .find(|next_argument| arguments_match(previous_argument, next_argument))
                .is_none()
        })
    }

    pub(crate) fn created_arguments(&self) -> impl Iterator<Item = &ast::Argument> {
        self.next_arguments().filter(move |next_argument| {
            self.previous_arguments()
                .find(|previous_argument| arguments_match(previous_argument, next_argument))
                .is_none()
        })
    }

    pub(crate) fn argument_pairs(&self) -> impl Iterator<Item = (&ast::Argument, &ast::Argument)> {
        self.previous_arguments().filter_map(move |previous_argument| {
            self.next_arguments()
                .find(|next_argument| arguments_match(previous_argument, next_argument))
                .map(|next_argument| (previous_argument, next_argument))
        })
    }

    fn previous_arguments(&self) -> impl Iterator<Item = &ast::Argument> {
        self.previous.properties.iter()
    }

    fn next_arguments(&self) -> impl Iterator<Item = &ast::Argument> {
        self.next.properties.iter()
    }
}

fn arguments_match(previous: &ast::Argument, next: &ast::Argument) -> bool {
    previous.name.name == next.name.name
}

#[cfg(test)]
mod tests {
    use super::super::{ModelDiffer, TopDiffer};
    use super::*;
    use datamodel::ast::parser::parse;

    fn custom_datasource_test_setup(test_fn: impl FnOnce(SourceArgumentsDiffer<'_>)) {
        let previous = r#"
        datasource mydb {
            provider = "somecustom"
            foo = "yes"
        }
        "#;
        let previous = parse(previous).unwrap();
        let next = r#"
        datasource mydb {
            provider = "somecustom"
            bar = "no"
        }
        "#;
        let next = parse(next).unwrap();

        let differ = TopDiffer {
            previous: &previous,
            next: &next,
        };

        let differ: SourceArgumentsDiffer<'_> = differ.updated_datasources().next().unwrap();

        test_fn(differ)
    }

    #[test]
    fn custom_datasource_test() {
        custom_datasource_test_setup(|diff| {
            let deleted_arguments = diff.deleted_arguments().collect::<Vec<_>>();

            assert_eq!(deleted_arguments.len(), 1);
            assert_eq!(deleted_arguments.get(0).unwrap().name.name, "foo");

            let created_arguments = diff.created_arguments().collect::<Vec<_>>();

            assert_eq!(created_arguments.len(), 1);
            assert_eq!(created_arguments.get(0).unwrap().name.name, "bar");
        })
    }
}
