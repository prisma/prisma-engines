use enumflags2::*;

macro_rules! tags {
    ($($name:ident = $pattern:expr,)*) => {
        /// Test-relevant connector tags.
        #[bitflags]
        #[derive(Copy, Clone, Debug, PartialEq)]
        #[repr(u32)]
        pub enum Tags {
            $($name = $pattern,)*
        }

        const ALL_TAG_NAMES: &[(&str, Tags)] = &[
            $(
                (stringify!($name), Tags::$name),
            )*
        ];
    }
}

tags![
    LowerCasesTableNames = 1 << 0,
    Mysql = 1 << 1,
    Mariadb = 1 << 2,
    Postgres = 1 << 3,
    Sqlite = 1 << 4,
    Mysql8 = 1 << 5,
    Mysql57 = 1 << 6,
    Mssql2017 = 1 << 7,
    Mssql2019 = 1 << 8,
    Postgres12 = 1 << 9,
    Mssql = 1 << 10,
    Vitess = 1 << 11,
    CockroachDb = 1 << 12,
    Postgres13 = 1 << 13,
    Postgres14 = 1 << 14,
    Postgres15 = 1 << 15,
    Postgres16 = 1 << 16,
    CockroachDb221 = 1 << 17,
    CockroachDb222 = 1 << 18,
    CockroachDb231 = 1 << 19,
];

pub fn tags_from_comma_separated_list(input: &str) -> BitFlags<Tags> {
    let mut tags = Default::default();

    for s in input.split(',').map(|s| s.trim()) {
        match ALL_TAG_NAMES.iter().find(|(name, _t)| name.eq_ignore_ascii_case(s)) {
            Some((_, tag)) => tags |= *tag,
            None => panic!("unknown tag: {s}"),
        }
    }

    tags
}
