
pub type TestResult = anyhow::Result<()>;

pub enum Runner {
    /// Using the QE crate directly for queries.
    Direct(DirectRunner),

    /// Using a NodeJS runner.
    NApi,

    /// Using the HTTP bridge
    Binary(),
}

impl Runner {
    pub fn load() -> Self {
        todo!()
    }

    pub fn run() ->
}

pub struct DirectRunner {}

