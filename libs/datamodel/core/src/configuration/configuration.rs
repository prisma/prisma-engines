use super::{Datasource, Generator};

pub struct Configuration {
    pub generators: Vec<Generator>,
    pub datasources: Vec<Datasource>,
}
