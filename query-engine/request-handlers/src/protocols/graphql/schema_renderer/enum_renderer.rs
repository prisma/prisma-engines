use super::*;

pub(crate) struct GqlEnumRenderer {
    enum_type: EnumType,
}

impl Renderer for GqlEnumRenderer {
    fn render(&self, ctx: &mut RenderContext) -> String {
        if ctx.already_rendered(&self.enum_type.name()) {
            return "".to_owned();
        }

        let values = self.format_enum_values();
        let rendered = format!("enum {} {{\n{}\n}}", self.enum_type.name(), values.join("\n"));

        ctx.add(self.enum_type.name(), rendered.clone());
        rendered
    }
}

impl GqlEnumRenderer {
    pub(crate) fn new(enum_type: EnumType) -> GqlEnumRenderer {
        GqlEnumRenderer { enum_type }
    }

    fn format_enum_values(&self) -> Vec<String> {
        match &self.enum_type {
            EnumType::String(s) => s.values().to_owned(),
            EnumType::Database(dbt) => dbt.external_values(),
            EnumType::FieldRef(f) => f.values(),
        }
    }
}
