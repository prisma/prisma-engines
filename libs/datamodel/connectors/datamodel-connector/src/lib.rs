use crate::scalars::ScalarType;

pub mod error;
pub mod scalars;

pub trait Connector {
    fn type_aliases(&self) -> &Vec<TypeAlias>;
    fn root_types(&self) -> &Vec<Box<dyn RootType>>;

    fn calculate_type(&self, name: &str, args: Vec<i32>) -> Type {
        // TODO: recurse through type constructors and find it
        match self.get_type_alias(name) {
            Some(alias) => self.calculate_type(&alias.aliased_to, args),
            None => {
                let root_type = self
                    .get_root_type(&name)
                    .expect(&format!("Did not find root type for name {}", &name));
                Type {
                    name: name.to_string(),
                    args,
                    root_type,
                }
            }
        }
    }

    fn get_type_alias(&self, name: &str) -> Option<&TypeAlias> {
        self.type_aliases().into_iter().find(|alias| &alias.name == name)
    }

    fn get_root_type(&self, name: &str) -> Option<&Box<dyn RootType>> {
        self.root_types().into_iter().find(|rt| rt.name() == name)
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

// TODO: it's unclear whether this should be indeed an interface
pub trait RootType {
    fn name(&self) -> &str;
    // represents the number of arguments for the type
    fn number_of_args(&self) -> usize;
    // calculates the underlying raw type
    fn raw_type(&self, args: &Vec<i32>) -> String;
    fn photon_type(&self) -> scalars::ScalarType;
}

struct SimpleRootType {
    name: String,
    raw_type: String,
    number_of_args: usize,
    photon_type: scalars::ScalarType,
}

impl SimpleRootType {
    pub fn without_args(name: &str, raw_type: &str, photon_type: scalars::ScalarType) -> SimpleRootType {
        SimpleRootType {
            name: name.to_string(),
            raw_type: raw_type.to_string(),
            photon_type,
            number_of_args: 0,
        }
    }

    pub fn with_args(
        name: &str,
        raw_type: &str,
        photon_type: scalars::ScalarType,
        number_of_args: usize,
    ) -> SimpleRootType {
        SimpleRootType {
            name: name.to_string(),
            raw_type: raw_type.to_string(),
            photon_type,
            number_of_args,
        }
    }
}

impl RootType for SimpleRootType {
    fn name(&self) -> &str {
        &self.name
    }

    fn number_of_args(&self) -> usize {
        self.number_of_args
    }

    fn raw_type(&self, args: &Vec<i32>) -> String {
        if self.number_of_args != args.len() {
            panic!(
                "Did not provide the required number of arguments. {} were required, but were {} provided.",
                self.number_of_args,
                args.len()
            )
        }
        if args.is_empty() {
            self.raw_type.to_string()
        } else {
            let args_as_strings: Vec<String> = args.iter().map(|x| x.to_string()).collect();
            self.raw_type.to_string() + "(" + &args_as_strings.join(",") + ")"
        }
    }

    fn photon_type(&self) -> ScalarType {
        self.photon_type
    }
}

// TODO: this might not be needed within this interface
pub struct Type<'a> {
    name: String,
    args: Vec<i32>,
    root_type: &'a Box<dyn RootType>,
}
impl Type<'_> {
    pub fn photon_type(&self) -> scalars::ScalarType {
        self.root_type.photon_type()
    }

    pub fn raw_type(&self) -> String {
        self.root_type.raw_type(&self.args)
    }
}

/// Postgres Example Impl
struct SimpleConnector {
    aliases: Vec<TypeAlias>,
    root_types: Vec<Box<dyn RootType>>,
}

impl SimpleConnector {
    pub fn postgres() -> SimpleConnector {
        let aliases = vec![
            // standard types
            TypeAlias::new("String", "Text"),
            //            TypeAlias::new("Boolean", "Boolean"),
            TypeAlias::new("Int", "Integer"),
            TypeAlias::new("String", "Text"),
            TypeAlias::new("String", "Text"),
            // custom types
            TypeAlias::new("Int8", "BigInt"),
            TypeAlias::new("Serial8", "BigSerial"),
            TypeAlias::new("Float8", "DoublePrecision"),
            TypeAlias::new("Int", "Integer"),
            TypeAlias::new("Int4", "Integer"),
            TypeAlias::new("Decimal", "Numeric"),
            TypeAlias::new("Float4", "Real"),
            TypeAlias::new("Int2", "SmallInt"),
            TypeAlias::new("Serial2", "SmallSerial"),
            TypeAlias::new("Serial4", "Serial"),
            TypeAlias::new("Char", "Character"),
            TypeAlias::new("VarChar", "CharacterVarying"),
            TypeAlias::new("TimestampTZ", "TimestampWithTimeZone"),
            TypeAlias::new("Bool", "Boolean"),
            TypeAlias::new("VarBit", "BitVarying"),
        ];
        /// missing because of interpolation:
        /// Numeric, Character, CharacterVarying, Timestamp, TimestampWithTimeZone, Time
        /// Bit, BitVarying
        ///
        /// types for which photon types are unclear:
        /// ByteA, Date, TimeTZ
        /// Point, Line, LSeg, Box, Path, Polygon, Circle
        /// CIDR, INet, Macaddr
        /// TSVector, TSQuery
        /// UUID
        /// XML, JSON, JSONB
        /// Int4Range, Int8Range, NumRange, TSRange, TSTZRange, DateRange
        /// TXIDSnapshot
        let root_types: Vec<Box<dyn RootType>> = vec![
            Box::new(SimpleRootType::without_args("BigInt", "bigint", ScalarType::Int)),
            Box::new(SimpleRootType::without_args("BigSerial", "bigserial", ScalarType::Int)),
            Box::new(SimpleRootType::without_args(
                "DoublePrecision",
                "double precision",
                ScalarType::Float,
            )),
            Box::new(SimpleRootType::without_args("Integer", "integer", ScalarType::Int)),
            Box::new(SimpleRootType::without_args("Real", "real", ScalarType::Float)),
            Box::new(SimpleRootType::without_args("SmallInt", "smallint", ScalarType::Int)),
            Box::new(SimpleRootType::without_args(
                "SmallSerial",
                "smallserial",
                ScalarType::Int,
            )),
            Box::new(SimpleRootType::without_args("Serial", "serial", ScalarType::Int)),
            Box::new(SimpleRootType::without_args("Money", "money", ScalarType::Float)),
            Box::new(SimpleRootType::without_args("Text", "text", ScalarType::String)),
            Box::new(SimpleRootType::without_args("Char", "char", ScalarType::String)),
            Box::new(SimpleRootType::without_args("Name", "name", ScalarType::String)),
            Box::new(SimpleRootType::without_args("Boolean", "boolean", ScalarType::Boolean)),
            Box::new(SimpleRootType::without_args("Boolean", "boolean", ScalarType::Boolean)),
            Box::new(SimpleRootType::without_args("PGLSN", "pg_lsn", ScalarType::Int)),
        ];
        SimpleConnector { aliases, root_types }
    }
}

impl Connector for SimpleConnector {
    fn type_aliases(&self) -> &Vec<TypeAlias> {
        &self.aliases
    }

    fn root_types(&self) -> &Vec<Box<dyn RootType>> {
        &self.root_types
    }
}
