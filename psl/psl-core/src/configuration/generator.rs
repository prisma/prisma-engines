use crate::{
    PreviewFeature,
    configuration::{EnvFunction, StringFromEnvVar},
};
use diagnostics::{Diagnostics, Span};
use enumflags2::BitFlags;
use parser_database::ast::Expression;
use schema_ast::ast::WithSpan;
use serde::{Serialize, Serializer, ser::SerializeSeq};
use std::collections::HashMap;

#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
pub enum GeneratorConfigValue {
    String(String),
    Array(Vec<GeneratorConfigValue>),
    Env(String),
}

impl From<String> for GeneratorConfigValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl GeneratorConfigValue {
    pub(crate) fn try_from_expression(expr: &Expression, diagnostics: &mut Diagnostics) -> Option<Self> {
        Some(match expr {
            Expression::NumericValue(val, _) => val.clone().into(),
            Expression::StringValue(val, _) => val.clone().into(),
            Expression::ConstantValue(val, _) => val.clone().into(),
            Expression::Function(name, _, _) if name == "env" => {
                let env_fn = EnvFunction::from_ast(expr, diagnostics)?;
                Self::Env(env_fn.var_name().to_owned())
            }
            Expression::Function(_, _, _) => Self::String(expr.to_string()),
            Expression::Array(elements, _) => Self::Array(
                elements
                    .iter()
                    .map(|element| Self::try_from_expression(element, diagnostics))
                    .collect::<Option<Vec<_>>>()?,
            ),
        })
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Generator {
    pub name: String,
    pub provider: StringFromEnvVar,
    pub output: Option<StringFromEnvVar>,
    pub config: HashMap<String, GeneratorConfigValue>,

    #[serde(default)]
    pub binary_targets: Vec<StringFromEnvVar>,

    #[serde(default, serialize_with = "mcf_preview_features")]
    pub preview_features: Option<BitFlags<PreviewFeature>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,

    #[serde(skip)]
    pub span: Span,
}

impl WithSpan for Generator {
    fn span(&self) -> Span {
        self.span
    }
}

pub fn mcf_preview_features<S>(feats: &Option<BitFlags<PreviewFeature>>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let feats = feats.unwrap_or_default();
    let mut seq = s.serialize_seq(Some(feats.len()))?;
    for feat in feats.iter() {
        seq.serialize_element(&feat)?;
    }
    seq.end()
}
