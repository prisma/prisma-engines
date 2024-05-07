//! Functions for fetching from quaint result rows

use quaint::connector::ResultRow;

pub trait Getter {
    fn get_expect_string(&self, name: &str) -> String;
    fn get_expect_char(&self, name: &str) -> char;
    fn get_expect_i64(&self, name: &str) -> i64;
    fn get_expect_bool(&self, name: &str) -> bool;

    fn get_string_array(&self, name: &str) -> Option<Vec<String>>;
    fn get_char(&self, name: &str) -> Option<char>;
    fn get_string(&self, name: &str) -> Option<String>;
    fn get_bool(&self, name: &str) -> Option<bool>;
    fn get_u32(&self, name: &str) -> Option<u32>;
    fn get_i64(&self, name: &str) -> Option<i64>;
}

impl Getter for ResultRow {
    #[track_caller]
    fn get_expect_string(&self, name: &str) -> String {
        self.get(name)
            .and_then(|x| x.to_string())
            .ok_or_else(|| format!("Getting {} from Resultrow {:?} as String failed", name, &self))
            .unwrap()
    }

    #[track_caller]
    fn get_expect_char(&self, name: &str) -> char {
        self.get(name)
            .and_then(|x| x.as_char())
            .ok_or_else(|| format!("Getting {} from Resultrow {:?} as char failed", name, &self))
            .unwrap()
    }

    #[track_caller]
    fn get_expect_i64(&self, name: &str) -> i64 {
        self.get(name)
            .and_then(|x| x.as_integer())
            .ok_or_else(|| format!("Getting {} from Resultrow {:?} as i64 failed", name, &self))
            .unwrap()
    }

    #[track_caller]
    fn get_expect_bool(&self, name: &str) -> bool {
        self.get_bool(name)
            .ok_or_else(|| format!("Getting {} from Resultrow {:?} as bool failed", name, &self))
            .unwrap()
    }

    fn get_string_array(&self, name: &str) -> Option<Vec<String>> {
        self.get(name).and_then(|x| x.to_vec::<String>())
    }

    fn get_char(&self, name: &str) -> Option<char> {
        self.get(name).and_then(|x| x.as_char())
    }

    // At least on MySQL, the encoding of booleans in the information schema
    // seems to be somewhat flexible, so we try to match "0", "1", 0 and 1
    // additionally. See https://github.com/prisma/prisma/issues/5235 for
    // example.
    fn get_bool(&self, name: &str) -> Option<bool> {
        self.get(name).and_then(|x| {
            x.as_bool()
                .or_else(|| {
                    x.as_i64().and_then(|n| match n {
                        0 => Some(false),
                        1 => Some(true),
                        _ => None,
                    })
                })
                .or_else(|| {
                    x.to_string().and_then(|s| match s.trim() {
                        "0" => Some(false),
                        "1" => Some(true),
                        _ => None,
                    })
                })
        })
    }

    fn get_string(&self, name: &str) -> Option<String> {
        self.get(name).and_then(|x| x.to_string())
    }

    fn get_u32(&self, name: &str) -> Option<u32> {
        self.get(name).and_then(|x| x.as_integer().map(|x| x as u32))
    }

    fn get_i64(&self, name: &str) -> Option<i64> {
        self.get(name).and_then(|x| x.as_integer())
    }
}
