use uuid::Uuid;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Id {
    String(String),
    Int(usize),
    UUID(Uuid),
}
