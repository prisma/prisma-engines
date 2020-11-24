//! Functions for parsing values from resultrows using regexes

use bigdecimal::BigDecimal;
use once_cell::sync::Lazy;
use prisma_value::PrismaValue;
use regex::Regex;
use std::str::FromStr;
use tracing::debug;

static RE_NUM: Lazy<Regex> = Lazy::new(|| Regex::new(r"^'?(-?\d+)'?$").expect("compile regex"));
static RE_FLOAT: Lazy<Regex> = Lazy::new(|| Regex::new(r"^'?([^']+)'?$").expect("compile regex"));

pub(crate) trait Parser {
    fn re_num() -> &'static Regex {
        &RE_NUM
    }

    fn re_float() -> &'static Regex {
        &RE_FLOAT
    }

    fn parse_int(value: &str) -> Option<PrismaValue> {
        let captures = Self::re_num().captures(value)?;
        let num_str = captures.get(1).expect("get capture").as_str();
        let num_rslt = num_str.parse::<i64>();
        match num_rslt {
            Ok(num) => Some(PrismaValue::Int(num)),
            Err(_) => None,
        }
    }

    fn parse_big_int(value: &str) -> Option<PrismaValue> {
        let captures = Self::re_num().captures(value)?;
        let num_str = captures.get(1).expect("get capture").as_str();
        let num_rslt = num_str.parse::<i64>();
        match num_rslt {
            Ok(num) => Some(PrismaValue::BigInt(num)),
            Err(_) => None,
        }
    }

    fn parse_bool(value: &str) -> Option<PrismaValue> {
        match value.to_lowercase().parse() {
            Ok(val) => Some(PrismaValue::Boolean(val)),
            Err(_) => None,
        }
    }

    fn parse_float(value: &str) -> Option<PrismaValue> {
        let captures = Self::re_float().captures(value)?;
        let num_str = captures.get(1).expect("get capture").as_str();

        match BigDecimal::from_str(num_str) {
            Ok(num) => Some(PrismaValue::Float(num)),
            Err(_) => {
                debug!("Couldn't parse float '{}'", value);
                None
            }
        }
    }

    fn unquote_string(value: &str) -> String {
        value
            .trim_start_matches('\'')
            .trim_end_matches('\'')
            .trim_start_matches('\\')
            .trim_start_matches('"')
            .trim_end_matches('"')
            .trim_end_matches('\\')
            .into()
    }
}

#[cfg(test)]
mod tests {
    use crate::unquote_string;

    #[test]
    fn unquoting_works() {
        let quoted_str = "'abc $$ def'".to_string();

        assert_eq!(unquote_string(&quoted_str), "abc $$ def");

        assert_eq!(unquote_string("heh "), "heh ");
    }
}
