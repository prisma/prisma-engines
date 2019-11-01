pub mod error;
pub mod scalars;

pub trait Connector {
    fn type_aliases(&self) -> &Vec<TypeAlias>;
    fn root_types(&self) -> &Vec<Box<dyn RootType>>;

    fn calculate_type(&self, name: String, args: Vec<i32>) -> Type {
        // TODO: recurse through type constructors and find it
        unimplemented!()
    }
}

pub struct TypeAlias {
    name: String,
    aliased_to: String,
}

// TODO: it's unclear whether this should be indeed an interface
trait RootType {
    fn name(&self) -> String;
    // represents the number of arguments for the type
    fn number_of_args(&self) -> i32;
    // calculates the underlying raw type
    fn raw_type(&self, args: &Vec<i32>) -> String;
    fn photon_type(&self) -> scalars::ScalarType;
}

// TODO: this might not be needed within this interface
pub struct Type {
    name: String,
    args: Vec<i32>,
    root_type: Box<dyn RootType>,
}
impl Type {
    fn photon_type(&self) -> scalars::ScalarType {
        self.root_type.photon_type()
    }

    fn raw_type(&self) -> String {
        self.root_type.raw_type(&self.args)
    }
}
