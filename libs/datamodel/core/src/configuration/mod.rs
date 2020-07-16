mod datasource;
mod generator;

pub use datasource::*;
pub use generator::*;

pub struct Configuration {
    pub generators: Vec<Generator>,
    pub datasources: Vec<Datasource>,
}
