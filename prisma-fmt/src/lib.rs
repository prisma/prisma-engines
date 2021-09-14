mod actions;
mod lint;
mod native;
mod preview;

#[cfg(target_arch = "wasm32")]
mod api {
    use crate::*;
    use datamodel::ast::reformat::Reformatter;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub fn format(schema: String) -> String {
        Reformatter::new(&schema).reformat_to_string()
    }

    #[wasm_bindgen]
    pub fn lint(schema: String) -> String {
        lint::run(&schema)
    }

    #[wasm_bindgen]
    pub fn native_types(schema: String) -> String {
        native::run(&schema)
    }

    #[wasm_bindgen]
    pub fn preview_features() -> String {
        preview::run()
    }

    #[wasm_bindgen]
    pub fn referential_actions(schema: String) -> String {
        actions::run(&schema)
    }
}
