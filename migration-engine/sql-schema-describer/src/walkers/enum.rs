use crate::{Enum, EnumId, EnumVariant, EnumVariantId, Walker};

/// Traverse an enum.
pub type EnumWalker<'a> = Walker<'a, EnumId>;

/// Traverse an enum variant.
pub type EnumVariantWalker<'a> = Walker<'a, EnumVariantId>;

impl<'a> EnumWalker<'a> {
    fn get(self) -> &'a Enum {
        &self.schema.enums[self.id.0 as usize]
    }

    /// The namespace the enum belongs to, if defined.
    pub fn namespace(self) -> Option<&'a str> {
        self.schema
            .namespaces
            .get(self.get().namespace_id.0 as usize)
            .map(|s| s.as_str())
    }

    /// The name of the enum. This is a made up name on MySQL.
    pub fn name(self) -> &'a str {
        &self.get().name
    }

    /// The variants of the enum.
    pub fn variants(self) -> impl ExactSizeIterator<Item = EnumVariantWalker<'a>> {
        super::range_for_key(&self.schema.enum_variants, self.id, |variant| variant.enum_id)
            .map(move |idx| self.walk(EnumVariantId(idx as u32)))
    }

    /// The names of the variants of the enum.
    pub fn values(self) -> impl ExactSizeIterator<Item = &'a str> {
        super::range_for_key(&self.schema.enum_variants, self.id, |variant| variant.enum_id)
            .map(move |idx| self.schema.enum_variants[idx].variant_name.as_str())
    }
}

impl<'a> EnumVariantWalker<'a> {
    fn get(self) -> &'a EnumVariant {
        &self.schema.enum_variants[self.id.0 as usize]
    }

    /// The parent enum.
    pub fn r#enum(self) -> EnumWalker<'a> {
        self.walk(self.get().enum_id)
    }

    /// The variant itself.
    pub fn name(self) -> &'a str {
        &self.get().variant_name
    }
}
