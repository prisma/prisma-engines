use crate::introspection::{
    datamodel_calculator::DatamodelCalculatorContext,
    sanitize_datamodel_names::{EnumVariantName, ModelName},
};
use indexmap::IndexMap;
use itertools::EitherOrBoth;
use psl::{
    parser_database::{self as db, walkers},
    schema_ast::ast::{WithDocumentation, WithName},
};
use sql_schema_describer as sql;
use std::borrow::Cow;

/// Pairing of a PSL enum to a database enum.
pub(crate) struct EnumPair<'a> {
    previous_and_next: EitherOrBoth<walkers::EnumWalker<'a>, sql::EnumWalker<'a>>,
    ctx: &'a DatamodelCalculatorContext<'a>,
}

impl<'a> EnumPair<'a> {
    pub fn from_model(previous: walkers::EnumWalker<'a>, ctx: &'a DatamodelCalculatorContext<'a>) -> Self {
        Self {
            previous_and_next: EitherOrBoth::Left(previous),
            ctx,
        }
    }

    pub fn from_db(db: sql::EnumWalker<'a>, ctx: &'a DatamodelCalculatorContext<'a>) -> Self {
        Self {
            previous_and_next: EitherOrBoth::Right(db),
            ctx,
        }
    }

    pub fn insert_model(&mut self, model: walkers::EnumWalker<'a>) {
        self.previous_and_next.insert_left(model);
    }

    /// The documentation on top of the enum.
    pub fn documentation(&self) -> Option<&'a str> {
        self.as_pair_ref().left().and_then(|enm| enm.ast_enum().documentation())
    }

    /// The mapped name, if defined, is the actual name of the enum in
    /// the database.
    pub fn mapped_name(&self) -> Option<&'a str> {
        self.ctx.enum_prisma_name(self.as_pair_ref().right()?.id).mapped_name()
    }

    /// Name of the enum in the PSL. The value can be sanitized if it
    /// contains characters that are not allowed in the PSL
    /// definition.
    pub fn name(&self) -> Cow<'a, str> {
        self.as_pair_ref()
            .map_any(
                |model_enum| model_enum.ast_enum().name().into(),
                |sql_enum| self.ctx.enum_prisma_name(sql_enum.id).prisma_name(),
            )
            .reduce(|_from_model, from_sql| from_sql)
    }

    /// The name of the variant is taken from the PSL.
    pub fn name_from_psl(&self) -> bool {
        matches!(
            self.as_pair_ref().right().map(|e| self.ctx.enum_prisma_name(e.id)),
            Some(ModelName::FromPsl {
                mapped_name: Some(_),
                ..
            })
        )
    }

    /// The namespace of the enumerator, if using the multi-schema feature.
    pub fn namespace(&self) -> Option<&'a str> {
        self.ctx
            .uses_namespaces()
            .then(|| self.as_pair_ref().right()?.explicit_namespace())
            .flatten()
    }

    /// The position of the enum from the PSL, if existing. Used for
    /// sorting the enums in the final introspected data model.
    pub fn previous_position(&self) -> Option<db::EnumId> {
        self.as_pair_ref().left().map(|e| e.id)
    }

    /// True, if enum uses the same name as another top-level item from
    /// a different namespace.
    pub fn uses_duplicate_name(&self) -> bool {
        self.as_pair_ref().left().is_none()
            && !self
                .as_pair_ref()
                .right()
                .is_some_and(|e| self.ctx.name_is_unique(e.name()))
    }

    /// The COMMENT of the enum.
    pub fn description(&self) -> Option<&'a str> {
        self.as_pair_ref().right()?.description()
    }

    /// True if we have a new enum and it has a comment.
    pub fn adds_a_description(&self) -> bool {
        self.as_pair_ref().left().is_none() && self.description().is_some()
    }

    /// Iterates all of the variants that are part of the enum.
    pub fn variants(&self) -> impl Iterator<Item = EnumVariantPair<'a>> + use<'a> {
        let mut variants = IndexMap::<Cow<'_, str>, EnumVariantPair<'a>>::new();

        // Start with the variants we found in the database.
        for variant in self.as_pair_ref().right().into_iter().flat_map(|enm| enm.variants()) {
            let name = self.ctx.enum_variant_name(variant.id);
            variants
                .entry(name.mapped_name().map(Cow::Borrowed).unwrap_or(name.prisma_name()))
                .or_insert(EnumVariantPair::from_db(variant, self.ctx));
        }

        let has_next = self.as_pair_ref().right().is_some();
        // Next, add the variants that we have in the model.
        for value in self.as_pair_ref().left().into_iter().flat_map(|enm| enm.values()) {
            let entry = variants.entry(value.database_name().into()).and_modify(|pair| {
                // If the variant has been found in the database, we insert its pair from the model.
                pair.previous_and_next.insert_left(value);
            });
            // If the variant was not found in the database, we insert it as a new pair, but only
            // if the introspection produced no enum at all (needed for databases where enums are
            // not natively supported).
            if !has_next {
                entry.or_insert(EnumVariantPair::from_model(value, self.ctx));
            }
        }

        variants.into_values()
    }

    fn as_pair_ref(&self) -> EitherOrBoth<&walkers::EnumWalker<'a>, &sql::EnumWalker<'a>> {
        self.previous_and_next.as_ref()
    }
}

/// Pairing of a PSL enum value a to database enum value.
pub struct EnumVariantPair<'a> {
    previous_and_next: EitherOrBoth<walkers::EnumValueWalker<'a>, sql::EnumVariantWalker<'a>>,
    ctx: &'a DatamodelCalculatorContext<'a>,
}

impl<'a> EnumVariantPair<'a> {
    fn from_model(previous: walkers::EnumValueWalker<'a>, ctx: &'a DatamodelCalculatorContext<'a>) -> Self {
        Self {
            previous_and_next: EitherOrBoth::Left(previous),
            ctx,
        }
    }

    fn from_db(db: sql::EnumVariantWalker<'a>, ctx: &'a DatamodelCalculatorContext<'a>) -> Self {
        Self {
            previous_and_next: EitherOrBoth::Right(db),
            ctx,
        }
    }

    /// The documentation on top of the enum.
    pub fn documentation(&self) -> Option<&'a str> {
        self.as_pair_ref().left().and_then(|variant| variant.documentation())
    }

    /// The mapped name, if defined, is the actual name of the variant in
    /// the database.
    pub fn mapped_name(&self) -> Option<&'a str> {
        self.ctx.enum_variant_name(self.as_pair_ref().right()?.id).mapped_name()
    }

    /// The name of the variant is taken from the PSL.
    pub fn name_from_psl(&self) -> bool {
        matches!(
            self.as_pair_ref().right().map(|e| self.ctx.enum_variant_name(e.id)),
            Some(EnumVariantName::FromPsl {
                mapped_name: Some(_),
                ..
            })
        )
    }

    /// Name of the variant in the PSL. The value can be sanitized if
    /// it contains characters that are not allowed in the PSL
    /// definition.
    pub fn name(&self) -> Cow<'a, str> {
        self.as_pair_ref()
            .map_any(
                |model_value| model_value.ast_value().name().into(),
                |sql_value| {
                    let n = self.ctx.enum_variant_name(sql_value.id).prisma_name();

                    // If the variant is sanitized as an empty string, we will
                    // comment the variant out and add a warning.
                    //
                    // The commented out variant cannot have an empty name, so we
                    // just print the non-sanitized one.
                    if n.is_empty() {
                        Cow::Borrowed(sql_value.name())
                    } else {
                        n
                    }
                },
            )
            .reduce(|_from_model, from_sql| from_sql)
    }

    fn as_pair_ref(&self) -> EitherOrBoth<&walkers::EnumValueWalker<'a>, &sql::EnumVariantWalker<'a>> {
        self.previous_and_next.as_ref()
    }
}
