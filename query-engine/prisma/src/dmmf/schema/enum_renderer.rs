use super::*;

pub struct DMMFEnumRenderer<'a> {
    enum_type: &'a EnumType,
}

impl<'a> Renderer<'a, ()> for DMMFEnumRenderer<'a> {
    fn render(&self, ctx: RenderContext) -> ((), RenderContext) {
        if ctx.already_rendered(self.enum_type.name()) {
            return ((), ctx);
        }

        let values = self.format_enum_values();

        let rendered = DMMFEnum {
            name: self.enum_type.name().to_owned(),
            values,
        };

        ctx.add_enum(self.enum_type.name().to_owned(), rendered);
        ((), ctx)
    }
}

impl<'a> DMMFEnumRenderer<'a> {
    pub fn new(enum_type: &'a EnumType) -> DMMFEnumRenderer<'a> {
        DMMFEnumRenderer { enum_type }
    }

    fn format_enum_values(&self) -> Vec<String> {
        match self.enum_type {
            EnumType::Internal(i) => i.values.clone(),
            EnumType::OrderBy(ord) => ord.values.iter().map(|(name, _)| name.to_owned()).collect(),
        }
    }
}
