use crate::query::*;
use dashmap::DashMap;
use log::debug;
use std::sync::{Arc, RwLock};

#[derive(Clone, Default)]
pub struct KnowledgeBase {
    shapes_to_prisma_queries: Arc<DashMap<RawQueryShape, RwLock<Vec<PrismaQuery>>>>,
    tags_to_raw_queries: Arc<DashMap<Tag, RawQuery>>,
    raw_queries_to_shapes: Arc<DashMap<RawQuery, RawQueryShape>>,
}

impl KnowledgeBase {
    pub fn index(&self, query_info: &SubmittedQueryInfo) -> Result<(), &'static str> {
        debug!("Indexing query information: {:?}", query_info);
        // index raw query
        self.tags_to_raw_queries
            .insert(query_info.tag.clone(), query_info.raw_query.clone());
        // index raw query shape
        let query_shape = RawQueryShape::from_raw_query(&query_info.raw_query);

        // index raw query
        self.raw_queries_to_shapes
            .insert(query_info.raw_query.clone(), query_shape.clone());

        if let Some(v) = self.shapes_to_prisma_queries.get(&query_shape) {
            if let Ok(mut v) = v.try_write() {
                v.push(query_info.prisma_query.clone());
            } else {
                return Err("There was a problem indexing the query");
            }
        } else {
            self.shapes_to_prisma_queries
                .insert(query_shape, RwLock::new(vec![query_info.prisma_query.clone()]));
        }
        Ok(())
    }

    pub fn get_tagged(&self, tag: Tag) -> Option<(RawQuery, Vec<PrismaQuery>)> {
        let raw_query = match self.tags_to_raw_queries.get(&tag) {
            Some(tag) => tag.clone(),
            None => return None,
        };
        let query_shape = match self.raw_queries_to_shapes.get(&raw_query) {
            Some(shape) => shape.clone(),
            None => return None,
        };

        let prisma_queries_lock = match self.shapes_to_prisma_queries.get(&query_shape) {
            Some(lock) => lock,
            None => return None,
        };
        let prisma_queries = prisma_queries_lock.read().unwrap().to_vec();
        Some((raw_query, prisma_queries.to_vec()))
    }
}

#[cfg(test)]
mod tests {
    use crate::kb::{KnowledgeBase, RawQueryShape, SubmittedQueryInfo};

    #[test]
    fn indexing() {
        const TAG: &str = "test";
        let prisma_query = "prisma.users.find_many(username: 'rosco')".to_string();
        let raw_query = "select * from users where username = 'rosco'".to_string();
        let query_shape = RawQueryShape("SELECT * FROM users WHERE username = ?".to_string());

        let kb = KnowledgeBase::default();
        let query_info = SubmittedQueryInfo {
            raw_query: raw_query.clone(),
            tag: TAG.to_string(),
            prisma_query: prisma_query.clone(),
        };
        assert!(kb.index(&query_info).is_ok());

        let q = kb.tags_to_raw_queries.get(TAG).expect("tag not found in prisma_idx");
        assert_eq!(raw_query, *q);

        let q = kb
            .raw_queries_to_shapes
            .get(raw_query.as_str())
            .expect("tag not found in prisma_idx");
        assert_eq!(query_shape, *q);

        let lock = kb
            .shapes_to_prisma_queries
            .get(&query_shape)
            .expect("query shape not found");
        let v = lock.read().unwrap();
        assert!(v.contains(&prisma_query));
    }
}
