use crate::query::*;
use dashmap::DashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, Default)]
pub(crate) struct KnowledgeBase {
    raw_shapes_idx: Arc<DashMap<RawQueryShape, RwLock<Vec<PrismaQuery>>>>,
    raw_idx: Arc<DashMap<Tag, RawQuery>>,
    prisma_idx: Arc<DashMap<Tag, RwLock<Vec<PrismaQuery>>>>,
}

impl KnowledgeBase {
    pub(crate) fn index(&self, query_info: &SubmittedQueryInfo) -> Result<(), &'static str> {
        // index raw query
        self.raw_idx
            .insert(query_info.tag.clone(), query_info.raw_query.clone());
        // index prisma query
        if let Some(v) = self.prisma_idx.get(&query_info.tag) {
            if let Ok(mut v) = v.try_write() {
                v.push(query_info.prisma_query.clone());
            } else {
                return Err("There was a problem indexing the query");
            }
        } else {
            self.prisma_idx.insert(
                query_info.tag.clone(),
                RwLock::new(vec![query_info.prisma_query.clone()]),
            );
        }
        // index raw query shape
        let query_shape = RawQueryShape::from_raw_query(&query_info.raw_query);
        if let Some(v) = self.raw_shapes_idx.get(&query_shape) {
            if let Ok(mut v) = v.try_write() {
                v.push(query_info.prisma_query.clone());
            } else {
                return Err("There was a problem indexing the query");
            }
        } else {
            self.raw_shapes_idx
                .insert(query_shape, RwLock::new(vec![query_info.prisma_query.clone()]));
        }
        Ok(())
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

        let lock = kb.prisma_idx.get(TAG).expect("tag not found in prisma_idx");
        let v = lock.read().unwrap();
        assert!(v.contains(&prisma_query));

        let q = kb.raw_idx.get(TAG).expect("tag not found in prisma_idx");
        assert_eq!(raw_query, *q);

        let lock = kb.raw_shapes_idx.get(&query_shape).expect("query shape not found");
        let v = lock.read().unwrap();
        assert!(v.contains(&prisma_query));
    }

    #[test]
    fn query_shape() {
        assert_eq!(
            "foo",
            RawQueryShape::from_raw_query("select * from foo where bar = 1").0
        );
    }
}
