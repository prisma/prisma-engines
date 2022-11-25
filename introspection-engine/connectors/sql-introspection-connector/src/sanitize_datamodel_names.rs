use once_cell::sync::Lazy;
use regex::Regex;
use std::borrow::Cow;

static RE_START: Lazy<Regex> = Lazy::new(|| Regex::new("^[^a-zA-Z]+").unwrap());
static RE: Lazy<Regex> = Lazy::new(|| Regex::new("[^_a-zA-Z0-9]").unwrap());

pub(crate) fn needs_sanitation(s: &str) -> bool {
    RE_START.is_match(s) || RE.is_match(s)
}

pub(crate) fn sanitize_string(s: &str) -> String {
    if needs_sanitation(s) {
        let start_cleaned = RE_START.replace_all(s, "");
        RE.replace_all(start_cleaned.as_ref(), "_").into_owned()
    } else {
        s.to_owned()
    }
}

/// Names that correspond to _types_ in the generated client. Concretely, enums, models and
/// composite types.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ModelName<'a> {
    FromPsl {
        name: &'a str,
        mapped_name: Option<&'a str>,
    },
    FromSql {
        name: &'a str,
    },
    RenamedReserved {
        mapped_name: &'a str,
    },
    RenamedSanitized {
        mapped_name: &'a str,
    },
}

impl<'a> ModelName<'a> {
    pub(crate) fn new_from_sql(name: &'a str) -> Self {
        match name {
            _ if psl::is_reserved_type_name(name) => ModelName::RenamedReserved { mapped_name: name },
            _ if crate::sanitize_datamodel_names::needs_sanitation(name) => {
                ModelName::RenamedSanitized { mapped_name: name }
            }
            name => ModelName::FromSql { name },
        }
    }

    pub(crate) fn prisma_name(&self) -> Cow<'a, str> {
        match self {
            ModelName::FromPsl { name, .. } => Cow::Borrowed(name),
            ModelName::FromSql { name } => Cow::Borrowed(name),
            ModelName::RenamedReserved { mapped_name } => Cow::Owned(format!("Renamed{mapped_name}")),
            ModelName::RenamedSanitized { mapped_name } => {
                Cow::Owned(crate::sanitize_datamodel_names::sanitize_string(mapped_name))
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum IntrospectedName<'a> {
    FromPsl {
        name: &'a str,
        mapped_name: Option<&'a str>,
    },
    FromSql {
        name: &'a str,
    },
    RenamedSanitized {
        mapped_name: &'a str,
    },
}

impl<'a> IntrospectedName<'a> {
    pub(crate) fn new_from_sql(name: &'a str) -> Self {
        match name {
            _ if crate::sanitize_datamodel_names::needs_sanitation(name) => {
                IntrospectedName::RenamedSanitized { mapped_name: name }
            }
            name => IntrospectedName::FromSql { name },
        }
    }

    pub(crate) fn prisma_name(&self) -> Cow<'a, str> {
        match self {
            IntrospectedName::FromPsl { name, .. } => Cow::Borrowed(name),
            IntrospectedName::FromSql { name } => Cow::Borrowed(name),
            IntrospectedName::RenamedSanitized { mapped_name } => {
                Cow::Owned(crate::sanitize_datamodel_names::sanitize_string(mapped_name))
            }
        }
    }

    pub(crate) fn mapped_name(&self) -> Option<&'a str> {
        match self {
            IntrospectedName::FromPsl { name: _, mapped_name } => *mapped_name,
            IntrospectedName::FromSql { name: _ } => None,
            IntrospectedName::RenamedSanitized { mapped_name } => Some(mapped_name),
        }
    }
}

pub(crate) enum EnumVariantName<'a> {
    Empty,
    RenamedSanitized {
        mapped_name: &'a str,
    },
    FromSql {
        name: &'a str,
    },
    FromPsl {
        name: &'a str,
        mapped_name: Option<&'a str>,
    },
}

impl<'a> EnumVariantName<'a> {
    pub(crate) fn new_from_sql(name: &'a str) -> Self {
        match name {
            "" => EnumVariantName::Empty,
            _ if crate::sanitize_datamodel_names::needs_sanitation(name) => {
                EnumVariantName::RenamedSanitized { mapped_name: name }
            }
            name => EnumVariantName::FromSql { name },
        }
    }

    pub(crate) fn prisma_name(&self) -> Cow<'a, str> {
        match self {
            EnumVariantName::Empty => Cow::Borrowed("EMPTY_ENUM_VALUE"),
            EnumVariantName::RenamedSanitized { mapped_name } => {
                Cow::Owned(crate::sanitize_datamodel_names::sanitize_string(mapped_name))
            }
            EnumVariantName::FromSql { name } | EnumVariantName::FromPsl { name, .. } => Cow::Borrowed(name),
        }
    }

    pub(crate) fn mapped_name(&self) -> Option<&'a str> {
        match self {
            EnumVariantName::Empty => Some(""),
            EnumVariantName::RenamedSanitized { mapped_name } => Some(mapped_name),
            EnumVariantName::FromSql { name: _ } => None,
            EnumVariantName::FromPsl { name: _, mapped_name } => *mapped_name,
        }
    }
}
