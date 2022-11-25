use crate::sanitize_datamodel_names;
use psl::{
    parser_database::walkers,
    schema_ast::ast::{self, WithDocumentation},
};
use sql_schema_describer as sql;
use std::borrow::Cow;

use super::Pair;

pub(crate) type EnumPair<'a> = Pair<'a, walkers::EnumWalker<'a>, sql::EnumWalker<'a>>;
pub(crate) type EnumVariantPair<'a> = Pair<'a, walkers::EnumValueWalker<'a>, sql::EnumVariantWalker<'a>>;

impl<'a> EnumPair<'a> {
    /// The position of the enum from the PSL, if existing. Used for
    /// sorting the enums in the final introspected data model.
    pub(crate) fn previous_position(self) -> Option<ast::EnumId> {
        self.previous.map(|e| e.id)
    }

    /// The namespace of the enumerator, if using the multi-schema feature.
    pub(crate) fn namespace(self) -> Option<&'a str> {
        if matches!(self.context.config.datasources.first(), Some(ds) if !ds.namespaces.is_empty()) {
            self.next.namespace()
        } else {
            None
        }
    }

    /// Name of the enum in the PSL. The value can be sanitized if it
    /// contains characters that are not allowed in the PSL
    /// definition.
    pub(crate) fn name(self) -> Cow<'a, str> {
        self.previous
            .map(|enm| Cow::Borrowed(enm.name()))
            .unwrap_or_else(|| match self.next.name() {
                name if psl::is_reserved_type_name(name) => Cow::Owned(format!("Renamed{name}")),
                name if sanitize_datamodel_names::needs_sanitation(name) => {
                    let sanitized = sanitize_datamodel_names::sanitize_string(name);

                    if sanitized.is_empty() {
                        Cow::Borrowed(name)
                    } else {
                        Cow::Owned(sanitized)
                    }
                }
                name => Cow::Borrowed(name),
            })
    }

    /// The mapped name, if defined, is the actual name of the enum in
    /// the database.
    pub(crate) fn mapped_name(self) -> Option<&'a str> {
        match self.previous {
            Some(enm) => enm.mapped_name(),
            None => match self.next.name() {
                name if psl::is_reserved_type_name(name) => Some(name),
                name if sanitize_datamodel_names::needs_sanitation(name) => Some(name),
                _ => None,
            },
        }
    }

    /// The documentation on top of the enum.
    pub(crate) fn documentation(self) -> Option<&'a str> {
        self.previous.and_then(|enm| enm.ast_enum().documentation())
    }

    /// Iterates all of the variants that are part of the enum.
    pub(crate) fn variants(self) -> impl ExactSizeIterator<Item = EnumVariantPair<'a>> + 'a {
        self.next.variants().map(move |next| {
            let variant_name = self.context.enum_variant_name(next.id);
            let prisma_name = variant_name.prisma_name();

            let previous = self.previous.and_then(|prev| {
                prev.values()
                    .find(|val| val.database_name() == variant_name.mapped_name().unwrap_or(&prisma_name))
            });

            Pair::new(self.context, previous, next)
        })
    }
}

impl<'a> EnumVariantPair<'a> {
    /// Name of the variant in the PSL. The value can be sanitized if
    /// it contains characters that are not allowed in the PSL
    /// definition.
    pub(crate) fn name(self) -> Cow<'a, str> {
        self.previous
            .map(|variant| Cow::Borrowed(variant.name()))
            .unwrap_or_else(|| match self.next.name() {
                name if name.is_empty() => Cow::Borrowed("EMPTY_ENUM_VALUE"),
                name if sanitize_datamodel_names::needs_sanitation(name) => {
                    let sanitized = sanitize_datamodel_names::sanitize_string(name);

                    if sanitized.is_empty() {
                        Cow::Borrowed(name)
                    } else {
                        Cow::Owned(sanitized)
                    }
                }
                name => Cow::Borrowed(name),
            })
    }

    /// The mapped name, if defined, is the actual name of the variant in
    /// the database.
    pub(crate) fn mapped_name(self) -> Option<&'a str> {
        match self.previous {
            Some(variant) => variant.mapped_name(),
            None => match self.next.name() {
                name if name.is_empty() => Some(name),
                name if sanitize_datamodel_names::needs_sanitation(name) => Some(name),
                _ => None,
            },
        }
    }

    /// The documentation on top of the enum.
    pub(crate) fn documentation(self) -> Option<&'a str> {
        self.previous.and_then(|variant| variant.documentation())
    }
}
