//! Render a datamodel to a string (for introspection).

mod render_configuration;
mod render_datamodel;

pub(crate) use self::{
    render_configuration::render_configuration,
    render_datamodel::{render_datamodel, RenderParams},
};

fn render_documentation(doc: &str, is_commented_out: bool, out: &mut String) {
    // We comment out objects in introspection. Those are put into `//` comments. We use the
    // documentation on the object to render an explanation for why that happened. It's nice if
    // this explanation is also in a `//` instead of a `///` comment.
    let prefix = if is_commented_out { "// " } else { "/// " };
    for line in doc.split('\n') {
        out.push_str(prefix);
        out.push_str(line);
        out.push('\n');
    }
}
