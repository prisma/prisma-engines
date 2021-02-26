use mongodb::bson::Document;
use prisma_models::RecordProjection;

#[derive(Debug, Default)]
pub(crate) struct CursorBuilder {
    cursor: RecordProjection,
}

impl CursorBuilder {
    pub fn new(cursor: RecordProjection) -> Self {
        Self { cursor }
    }

    pub fn build(self) -> crate::Result<Document> {
        todo!()
    }
}
