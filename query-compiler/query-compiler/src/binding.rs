use std::borrow::Cow;

use query_core::NodeRef;
use query_structure::SelectedField;

const JOIN_PARENT: &str = "@parent";
const DEFAULTS: &str = "@defaults";

const FIELD_SEPARATOR: &str = "$";

pub fn node_result(node: NodeRef) -> Cow<'static, str> {
    node.id().into()
}

pub fn projected_dependency(source: NodeRef, field: &SelectedField) -> Cow<'static, str> {
    format!("{}{FIELD_SEPARATOR}{}", source.id(), field.prisma_name()).into()
}

pub fn join_parent() -> Cow<'static, str> {
    Cow::Borrowed(JOIN_PARENT)
}

pub fn join_parent_field(field: &SelectedField) -> Cow<'static, str> {
    format!("{JOIN_PARENT}{FIELD_SEPARATOR}{}", field.prisma_name()).into()
}

pub fn defaults() -> Cow<'static, str> {
    Cow::Borrowed(DEFAULTS)
}
