use datamodel::ast;

pub(crate) struct DirectiveArgumentDiffer<'a> {
    pub(crate) previous: &'a ast::DirectiveArgument,
    pub(crate) next: &'a ast::DirectiveArgument,
}

impl<'a> DirectiveArgumentDiffer<'a> {
    fn created_values() {
        unimplemented!()
    }

    fn deleted_values() {
        unimplemented!()
    }

    fn changed_single_value() -> Option<()> {
        unimplemented!()
    }
}

struct ArgumentDiff {
    changed_values: (usize, ast::Value),
    added_values: (usize, ast::Value),
    deleted_values: (usize, ast::Value),
}
