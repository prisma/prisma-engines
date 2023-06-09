use super::*;

pub(crate) fn render_enum_types<'a>(ctx: &mut RenderContext, enum_types: impl Iterator<Item = EnumType> + 'a) {
    let mut borrows: Vec<_> = enum_types.collect();

    borrows.sort_by_key(|a| a.name());
    borrows.into_iter().for_each(|et| DmmfEnumRenderer::new(et).render(ctx));
}

pub struct DmmfEnumRenderer {
    enum_type: EnumType,
}

impl<'a> Renderer<'a> for DmmfEnumRenderer {
    fn render(&self, ctx: &mut RenderContext) {
        let ident = self.enum_type.identifier();
        if ctx.already_rendered(ident) {
            return;
        }

        let values = self.format_enum_values();
        let rendered = DmmfEnum {
            name: self.enum_type.name(),
            values,
        };

        ctx.add_enum(ident.clone(), rendered);
    }
}

impl DmmfEnumRenderer {
    pub(crate) fn new(enum_type: EnumType) -> DmmfEnumRenderer {
        DmmfEnumRenderer { enum_type }
    }

    fn format_enum_values(&self) -> Vec<String> {
        match &self.enum_type {
            EnumType::String(s) => s.values().to_owned(),
            EnumType::Database(dbt) => dbt.external_values(),
            EnumType::FieldRef(f) => f.values(),
        }
    }
}
