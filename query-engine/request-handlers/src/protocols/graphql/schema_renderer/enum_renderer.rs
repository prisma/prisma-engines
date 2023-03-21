use super::*;

pub struct GqlEnumRenderer<'a> {
    enum_type: &'a EnumType,
}

impl<'a> Renderer for GqlEnumRenderer<'a> {
    fn render(&self, ctx: &mut RenderContext) -> String {
        if ctx.already_rendered(self.enum_type.name()) {
            return "".to_owned();
        }

        let values = self.format_enum_values();
        let rendered = format!("enum {} {{\n{}\n}}", self.enum_type.name(), values.join("\n"));

        ctx.add(self.enum_type.name().to_owned(), rendered.clone());
        rendered
    }
}

impl<'a> GqlEnumRenderer<'a> {
    pub fn new(enum_type: &EnumType) -> GqlEnumRenderer {
        GqlEnumRenderer { enum_type }
    }

    fn format_enum_values(&self) -> Vec<String> {
        match self.enum_type {
            EnumType::String(s) => s.values().to_owned(),
            EnumType::Database(dbt) => dbt.external_values(),
            EnumType::FieldRef(f) => f.values(),
        }
    }
}
