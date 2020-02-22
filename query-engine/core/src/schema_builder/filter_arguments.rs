use super::*;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct FilterArgument {
    pub suffix: &'static str,
    pub is_list: bool,
}

/// Wrapper type for static initialization
struct StaticFilterArguments {
    pub base: &'static [FilterArgument],
    pub inclusion: &'static [FilterArgument],
    pub alphanumeric: &'static [FilterArgument],
    pub string: &'static [FilterArgument],
    pub multi_relation: &'static [FilterArgument],
    pub one_relation: &'static [FilterArgument],
}

static FILTER_ARGUMENTS: StaticFilterArguments = StaticFilterArguments {
    base: &[
        FilterArgument { suffix: "", is_list: false },
        FilterArgument { suffix: "_not",is_list: false }, // All values that are not equal to given value.
    ],

    inclusion: &[
        FilterArgument { suffix: "_in",is_list: true }, // All values that are contained in given list.
        FilterArgument { suffix: "_not_in",is_list: true } // All values that are not contained in given list.
    ],

    alphanumeric: &[
        FilterArgument { suffix: "_lt",is_list: false }, // All values less than the given value.
        FilterArgument { suffix: "_lte",is_list: false }, // All values less than or equal the given value.
        FilterArgument { suffix: "_gt",is_list: false }, // All values greater than the given value.
        FilterArgument { suffix: "_gte",is_list: false } // All values greater than or equal the given value.
    ],

    string: &[
        FilterArgument { suffix: "_contains",is_list: false }, // All values containing the given string.
        FilterArgument { suffix: "_not_contains",is_list: false }, // All values not containing the given string.
        FilterArgument { suffix: "_starts_with",is_list: false }, // All values starting with the given string.
        FilterArgument { suffix: "_not_starts_with",is_list: false }, // All values not starting with the given string.
        FilterArgument { suffix: "_ends_with",is_list: false }, // All values ending with the given string.
        FilterArgument { suffix: "_not_ends_with",is_list: false } // All values not ending with the given string.
    ],

    multi_relation: &[
        FilterArgument { suffix: "_every",is_list: false }, // All records where all records in the relation satisfy the given condition.
        FilterArgument { suffix: "_some",is_list: false }, // All records that have at least one record in the relation satisfying the given condition.
        FilterArgument { suffix: "_none",is_list: false } // All records that have no record in the relation satisfying the given condition.
    ],

    one_relation: &[FilterArgument { suffix: "", is_list: false }],
};

pub fn get_field_filters<'a>(field: &ModelField) -> Vec<&'a FilterArgument> {
    let args = &FILTER_ARGUMENTS;

    let filters = match field {
        ModelField::Relation(_) if field.is_list() => vec![&args.multi_relation],
        ModelField::Scalar(_) if field.is_list() => vec![],
        ModelField::Relation(_) => vec![&args.one_relation],
        ModelField::Scalar(sf) => match sf.type_identifier {
            TypeIdentifier::UUID => vec![&args.base, &args.inclusion, &args.alphanumeric, &args.string],
            TypeIdentifier::String => vec![&args.base, &args.inclusion, &args.alphanumeric, &args.string],
            TypeIdentifier::Int => vec![&args.base, &args.inclusion, &args.alphanumeric],
            TypeIdentifier::Float => vec![&args.base, &args.inclusion, &args.alphanumeric],
            TypeIdentifier::Boolean => vec![&args.base],
            TypeIdentifier::Enum => vec![&args.base, &args.inclusion],
            TypeIdentifier::DateTime => vec![&args.base, &args.inclusion, &args.alphanumeric],
            TypeIdentifier::Json => vec![],
        },
    };

    filters
        .into_iter()
        .map(|l| l.iter().collect::<Vec<&'a FilterArgument>>())
        .flatten()
        .collect()
}
