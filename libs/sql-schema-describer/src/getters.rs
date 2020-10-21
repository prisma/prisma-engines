//! Functions for fetching from quaint result rows

use quaint::connector::ResultRow;

pub trait Getter {
    fn get_expect_string(&self, name: &str) -> String;
    fn get_expect_char(&self, name: &str) -> char;
    fn get_expect_i64(&self, name: &str) -> i64;
    fn get_expect_bool(&self, name: &str) -> bool;

    fn get_string(&self, name: &str) -> Option<String>;
    fn get_u32(&self, name: &str) -> Option<u32>;
    fn get_i64(&self, name: &str) -> Option<i64>;
}

impl Getter for ResultRow {
    fn get_expect_string(&self, name: &str) -> String {
        self.get(name)
            .and_then(|x| x.to_string())
            .ok_or_else(|| format!("Getting {} from Resultrow {:?} as String failed", name, &self))
            .unwrap()
    }

    fn get_expect_char(&self, name: &str) -> char {
        self.get(name)
            .and_then(|x| x.as_char())
            .ok_or_else(|| format!("Getting {} from Resultrow {:?} as char failed", name, &self))
            .unwrap()
    }

    fn get_expect_i64(&self, name: &str) -> i64 {
        self.get(name)
            .and_then(|x| x.as_i64())
            .ok_or_else(|| format!("Getting {} from Resultrow {:?} as i64 failed", name, &self))
            .unwrap()
    }

    fn get_expect_bool(&self, name: &str) -> bool {
        self.get(name)
            .and_then(|x| x.as_bool())
            .ok_or_else(|| format!("Getting {} from Resultrow {:?} as bool failed", name, &self))
            .unwrap()
    }

    fn get_string(&self, name: &str) -> Option<String> {
        self.get(name).and_then(|x| x.to_string())
    }

    fn get_u32(&self, name: &str) -> Option<u32> {
        self.get(name).and_then(|x| x.as_i64().map(|x| x as u32))
    }

    fn get_i64(&self, name: &str) -> Option<i64> {
        self.get(name).and_then(|x| x.as_i64())
    }
}
