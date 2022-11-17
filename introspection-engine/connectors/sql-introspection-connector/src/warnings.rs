use introspection_connector::Warning;
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub(crate) struct Model {
    pub(crate) model: String,
}

#[derive(Serialize, Debug, Clone)]
pub(crate) struct Enum {
    pub(crate) enm: String,
}

impl Enum {
    pub(crate) fn new(name: &str) -> Self {
        Enum { enm: name.to_owned() }
    }
}

#[derive(Serialize, Debug, Clone)]
pub(crate) struct ModelAndField {
    pub(crate) model: String,
    pub(crate) field: String,
}

#[derive(Serialize, Debug, Clone)]
pub(crate) struct ModelAndIndex {
    pub(crate) model: String,
    pub(crate) index_db_name: String,
}

#[derive(Serialize, Debug)]
pub(crate) struct ModelAndFieldAndType {
    pub(crate) model: String,
    pub(crate) field: String,
    pub(crate) tpe: String,
}

#[derive(Serialize, Debug, Clone)]
pub(crate) struct EnumAndValue {
    pub(crate) enm: String,
    pub(crate) value: String,
}

pub(crate) fn warning_models_without_identifier(affected: &[Model]) -> Warning {
    Warning {
        code: 1,
        message: "The following models were commented out as they do not have a valid unique identifier or id. This is currently not supported by the Prisma Client.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(crate) fn warning_unsupported_types(affected: &[ModelAndFieldAndType]) -> Warning {
    Warning {
        code: 3,
        message: "These fields are not supported by the Prisma Client, because Prisma currently does not support their types.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(crate) fn warning_default_cuid_warning(affected: &[ModelAndField]) -> Warning {
    Warning {
        code: 5,
        message:
            "These id fields had a `@default(cuid())` added because we believe the schema was created by Prisma 1."
                .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(crate) fn warning_default_uuid_warning(affected: &[ModelAndField]) -> Warning {
    Warning {
        code: 6,
        message:
            "These id fields had a `@default(uuid())` added because we believe the schema was created by Prisma 1."
                .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(crate) fn warning_enriched_with_map_on_model(affected: &[Model]) -> Warning {
    Warning {
        code: 7,
        message: "These models were enriched with `@@map` information taken from the previous Prisma schema.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(crate) fn warning_enriched_with_map_on_field(affected: &[ModelAndField]) -> Warning {
    Warning {
        code: 8,
        message: "These fields were enriched with `@map` information taken from the previous Prisma schema.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(crate) fn warning_enriched_with_map_on_enum(affected: &[Enum]) -> Warning {
    Warning {
        code: 9,
        message: "These enums were enriched with `@@map` information taken from the previous Prisma schema.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(crate) fn warning_enriched_with_map_on_enum_value(affected: &[EnumAndValue]) -> Warning {
    Warning {
        code: 10,
        message: "These enum values were enriched with `@map` information taken from the previous Prisma schema."
            .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

//todo maybe we can get rid of this alltogether due to @@ignore
//but maybe we should have warnings for ignored fields and models
pub(crate) fn warning_models_without_columns(affected: &[Model]) -> Warning {
    Warning {
        code: 14,
        message: "The following models were commented out as we could not retrieve columns for them. Please check your privileges.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(crate) fn warning_enriched_with_custom_primary_key_names(affected: &[Model]) -> Warning {
    Warning {
        code: 18,
        message: "These models were enriched with custom compound id names taken from the previous Prisma schema."
            .into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}

pub(crate) fn warning_relations_added_from_the_previous_data_model(affected: &[Model]) -> Warning {
    Warning {
        code: 19,
        message: "Relations were copied from the previous data model due to not using foreign keys in the database. If any of the relation columns changed in the database, the relations might not be correct anymore.".into(),
        affected: serde_json::to_value(affected).unwrap(),
    }
}
