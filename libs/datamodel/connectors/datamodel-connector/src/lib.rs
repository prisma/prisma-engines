use crate::scalars::ScalarType;

pub mod error;
mod example_connector;
pub mod scalars;

pub trait Connector {
    fn type_aliases(&self) -> &Vec<TypeAlias>;
    fn field_type_constructors(&self) -> &Vec<FieldTypeConstructor>;

    fn calculate_type(&self, name: &str, args: Vec<i32>) -> FieldType {
        // TODO: recurse through type constructors and find it
        match self.get_type_alias(name) {
            Some(alias) => self.calculate_type(&alias.aliased_to, args),
            None => {
                let constructor = self
                    .get_field_type_constructor(&name)
                    .expect(&format!("Did not find type constructor for name {}", &name));
                FieldType {
                    name: name.to_string(),
                    args,
                    constructor,
                }
            }
        }
    }

    fn get_type_alias(&self, name: &str) -> Option<&TypeAlias> {
        self.type_aliases().into_iter().find(|alias| &alias.name == name)
    }

    fn get_field_type_constructor(&self, name: &str) -> Option<&FieldTypeConstructor> {
        self.field_type_constructors().into_iter().find(|rt| rt.name() == name)
    }
}

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

pub struct FieldTypeConstructor {
    name: String,
    datasource_type: String,
    number_of_args: usize,
    prisma_type: scalars::ScalarType,
}

impl FieldTypeConstructor {
    pub fn without_args(name: &str, datasource_type: &str, prisma_type: scalars::ScalarType) -> FieldTypeConstructor {
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
        prisma_type: scalars::ScalarType,
        number_of_args: usize,
    ) -> FieldTypeConstructor {
        FieldTypeConstructor {
            name: name.to_string(),
            datasource_type: datasource_type.to_string(),
            prisma_type,
            number_of_args,
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn number_of_args(&self) -> usize {
        self.number_of_args
    }

    fn datasource_type(&self, args: &Vec<i32>) -> String {
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

    fn prisma_type(&self) -> ScalarType {
        self.prisma_type
    }
}

// TODO: this might not be needed within this interface
pub struct FieldType<'a> {
    name: String,
    args: Vec<i32>,
    constructor: &'a FieldTypeConstructor,
}
impl FieldType<'_> {
    pub fn prisma_type(&self) -> scalars::ScalarType {
        self.constructor.prisma_type()
    }

    pub fn datasource_type(&self) -> String {
        self.constructor.datasource_type(&self.args)
    }
}
