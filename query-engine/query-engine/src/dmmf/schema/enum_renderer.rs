use super::*;

pub struct DMMFEnumRenderer {
    enum_type: EnumType,
}

impl Renderer for DMMFEnumRenderer {
    fn render(&self, ctx: &mut RenderContext) {
        if ctx.already_rendered(self.enum_type.name()) {
            return;
        }

        let values = self.format_enum_values();

        let rendered = DMMFEnum {
            name: self.enum_type.name().to_owned(),
            values,
        };

        ctx.add_enum(self.enum_type.name().to_owned(), rendered);
    }
}

impl<'a> DMMFEnumRenderer {
    pub fn new(enum_type: &EnumType) -> DMMFEnumRenderer {
        DMMFEnumRenderer {
            enum_type: enum_type.clone(),
        }
    }

    fn format_enum_values(&self) -> Vec<String> {
        match &self.enum_type {
            EnumType::String(s) => s.values().to_owned(),
            EnumType::Internal(i) => i.external_values(),
            EnumType::FieldRef(f) => f.values(),
        }
    }
}
