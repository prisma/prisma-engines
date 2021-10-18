mod actions;
mod lint;
mod native;
mod preview;

pub fn format(schema: String) -> String {
    use datamodel::ast::reformat::Reformatter;
    Reformatter::new(&schema).reformat_to_string()
}

pub fn lint(schema: String) -> String {
    lint::run(&schema)
}

pub fn native_types(schema: String) -> String {
    native::run(&schema)
}

pub fn preview_features() -> String {
    preview::run()
}

pub fn referential_actions(schema: String) -> String {
    actions::run(&schema)
}
