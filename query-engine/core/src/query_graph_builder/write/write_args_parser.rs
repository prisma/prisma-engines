use super::*;
use crate::query_document::{ParsedInputMap, ParsedInputValue};
use connector::WriteArgs;
use prisma_models::{Field, ModelRef, PrismaValue, RelationFieldRef, TypeIdentifier};
use std::{convert::TryInto, sync::Arc};

#[derive(Default, Debug)]
pub struct WriteArgsParser {
    pub args: WriteArgs,
    pub nested: Vec<(RelationFieldRef, ParsedInputMap)>,
}

impl WriteArgsParser {
    /// Creates a new set of WriteArgsParser. Expects the parsed input map from the respective data key, not the enclosing map.
    /// E.g.: { data: { THIS MAP } } from the `data` argument of a write query.
    pub fn from(model: &ModelRef, data_map: ParsedInputMap) -> QueryGraphBuilderResult<Self> {
        data_map.into_iter().try_fold(
            WriteArgsParser::default(),
            |mut args, (k, v): (String, ParsedInputValue)| {
                let field = model.fields().find_from_all(&k).unwrap();
                match field {
                    Field::Scalar(sf) if sf.is_list => {
                        let vals: ParsedInputMap = v.try_into()?;
                        let set_value = vals.into_iter().find(|(k, _)| k == "set");

                        let set_value: PrismaValue = match set_value {
                            Some(value) => value.1.try_into()?,
                            None => {
                                return Err(QueryGraphBuilderError::InputError(format!(
                                    "The `set` argument was not provided for field `{field_name}` on `{model_name}`",
                                    field_name = &sf.name,
                                    model_name = &model.name,
                                )))
                            }
                        };

                        args.args.insert(sf.db_name().clone(), set_value)
                    }
                    Field::Scalar(sf) => {
                        match &sf.type_identifier {
                            TypeIdentifier::Enum(enum_name) => {
                                //todo
                                // the typeidentifier needs the actual enum it is referring to
                                let value_as_string: Option<String> = v.try_into()?;

                                let internal_datamodel = model.internal_data_model.upgrade();
                                let inum = internal_datamodel.unwrap();
                                let v = inum.enums.iter().find(|inum| inum.name == *enum_name).unwrap();

                                let enum_value = match value_as_string {
                                    Some(value) => v.values.iter().find(|iv| iv.name == value),
                                    None => None,
                                };

                                let value: PrismaValue = match enum_value {
                                    Some(en_value) => {
                                        let value = en_value.database_name.clone().unwrap_or(en_value.name.clone());
                                        PrismaValue::Enum(value)
                                    }
                                    None => PrismaValue::Null, // todo correct???
                                };

                                args.args.insert(sf.db_name().clone(), value)
                            }
                            _ => {
                                let value: PrismaValue = v.try_into()?;
                                args.args.insert(sf.db_name().clone(), value)
                            }
                        }
                    }

                    Field::Relation(ref rf) => {
                        args.nested.push((Arc::clone(rf), v.try_into()?));
                    }
                };

                Ok(args)
            },
        )
    }
}
