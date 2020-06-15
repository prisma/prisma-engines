use introspection_connector::Warning;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct Model {
    pub(crate) model: String,
}

#[derive(Serialize, Debug)]
pub struct ModelAndField {
    pub(crate) model: String,
    pub(crate) field: String,
}

#[derive(Serialize, Debug)]
pub struct ModelAndFieldAndType {
    pub(crate) model: String,
    pub(crate) field: String,
    pub(crate) tpe: String,
}

#[derive(Serialize, Debug)]
pub struct EnumAndValue {
    pub(crate) enm: String,
    pub(crate) value: String,
}

pub fn warning_models_without_identifier(affected: &Vec<Model>) -> Warning {
    Warning {
        code: 1,
        message: "These models do not have a unique identifier or id and are therefore commented out.".into(),
        affected: serde_json::to_value(&affected).unwrap(),
    }
}

pub fn warning_fields_with_empty_names(affected: &Vec<ModelAndField>) -> Warning {
    Warning {
        code: 2,
        message: "These fields were commented out because of invalid names. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]*."
            .into(),
        affected: serde_json::to_value(&affected).unwrap(),
    }
}

pub fn warning_unsupported_types(affected: &Vec<ModelAndFieldAndType>) -> Warning {
    Warning {
        code: 3,
        message: "These fields were commented out because we currently do not support their types.".into(),
        affected: serde_json::to_value(&affected).unwrap(),
    }
}

pub fn warning_enum_values_with_empty_names(affected: &Vec<EnumAndValue>) -> Warning {
    Warning {
        code: 4,
        message: "These enum values were commented out because of invalid names. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]*."
            .into(),
        affected: serde_json::to_value(&affected).unwrap(),
    }
}

pub fn warning_default_cuid_warning(affected: &Vec<ModelAndField>) -> Warning {
    Warning {
        code: 5,
        message:
            "These id fields had a `@default(cuid())` added because we believe the schema was created by Prisma 1."
                .into(),
        affected: serde_json::to_value(&affected).unwrap(),
    }
}

pub fn warning_default_uuid_warning(affected: &Vec<ModelAndField>) -> Warning {
    Warning {
        code: 6,
        message:
            "These id fields had a `@default(uuid())` added because we believe the schema was created by Prisma 1."
                .into(),
        affected: serde_json::to_value(&affected).unwrap(),
    }
}
