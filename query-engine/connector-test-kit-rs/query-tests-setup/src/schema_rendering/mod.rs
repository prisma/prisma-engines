mod mongodb_renderer;
mod sql_renderer;

pub use mongodb_renderer::*;
pub use sql_renderer::*;

use crate::{IdFragment, SchemaFragment};

pub trait SchemaRenderer {
    fn render(&self, fragment: SchemaFragment) -> String {
        match fragment {
            SchemaFragment::Id(id) => self.render_id(id),
        }
    }

    fn render_id(&self, id: IdFragment) -> String;
}
