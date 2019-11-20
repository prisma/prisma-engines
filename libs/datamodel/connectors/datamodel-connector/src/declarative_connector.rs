use super::{Connector, ScalarFieldType, ScalarType};

#[derive(Debug)]
pub struct DeclarativeConnector {
    pub type_aliases: Vec<TypeAlias>,
    pub field_type_constructors: Vec<FieldTypeConstructor>,
}

impl Connector for DeclarativeConnector {
    fn calculate_type(&self, name: &str, args: Vec<i32>) -> Option<ScalarFieldType> {
        match self.get_type_alias(name) {
            Some(alias) => self.calculate_type(&alias.aliased_to, args),
            None => self.get_field_type_constructor(&name).map(|constructor| {
                let datasource_type = constructor.datasource_type(&args);

                ScalarFieldType {
                    name: name.to_string(),
                    prisma_type: constructor.prisma_type,
                    datasource_type,
                }
            }),
        }
    }
}

impl DeclarativeConnector {
    fn get_type_alias(&self, name: &str) -> Option<&TypeAlias> {
        self.type_aliases.iter().find(|alias| &alias.name == name)
    }

    fn get_field_type_constructor(&self, name: &str) -> Option<&FieldTypeConstructor> {
        self.field_type_constructors.iter().find(|rt| &rt.name == name)
    }
}

#[derive(Debug)]
pub struct TypeAlias {
    name: String,
    aliased_to: String,
}
impl TypeAlias {
    pub fn new(name: &str, aliased_to: &str) -> TypeAlias {
        TypeAlias {
            name: name.to_string(),
            aliased_to: aliased_to.to_string(),
        }
    }
}

#[derive(Debug)]
pub struct FieldTypeConstructor {
    name: String,
    datasource_type: String,
    number_of_args: usize,
    prisma_type: ScalarType,
}

impl FieldTypeConstructor {
    pub fn without_args(name: &str, datasource_type: &str, prisma_type: ScalarType) -> FieldTypeConstructor {
        FieldTypeConstructor {
            name: name.to_string(),
            datasource_type: datasource_type.to_string(),
            prisma_type,
            number_of_args: 0,
        }
    }

    pub fn with_args(
        name: &str,
        datasource_type: &str,
        prisma_type: ScalarType,
        number_of_args: usize,
    ) -> FieldTypeConstructor {
        FieldTypeConstructor {
            name: name.to_string(),
            datasource_type: datasource_type.to_string(),
            prisma_type,
            number_of_args,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn number_of_args(&self) -> usize {
        self.number_of_args
    }

    pub fn datasource_type(&self, args: &Vec<i32>) -> String {
        if self.number_of_args != args.len() {
            panic!(
                "Did not provide the required number of arguments. {} were required, but were {} provided.",
                self.number_of_args,
                args.len()
            )
        }
        if args.is_empty() {
            self.datasource_type.to_string()
        } else {
            let args_as_strings: Vec<String> = args.iter().map(|x| x.to_string()).collect();
            self.datasource_type.to_string() + "(" + &args_as_strings.join(",") + ")"
        }
    }

    pub fn prisma_type(&self) -> ScalarType {
        self.prisma_type
    }
}
