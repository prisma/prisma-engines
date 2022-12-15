use super::SqlSchemaDifferFlavour;
use crate::{
    flavour::MysqlFlavour,
    pair::Pair,
    sql_schema_differ::{all_match, ColumnTypeChange},
};
use psl::builtin_connectors::MySqlType;
use sql_schema_describer::{
    walkers::{ColumnWalker, IndexWalker},
    ColumnTypeFamily,
};

impl SqlSchemaDifferFlavour for MysqlFlavour {
    fn can_rename_foreign_key(&self) -> bool {
        false
    }

    fn can_rename_index(&self) -> bool {
        !self.is_mariadb() && !self.is_mysql_5_6()
    }

    fn can_cope_with_foreign_key_column_becoming_non_nullable(&self) -> bool {
        false
    }

    fn column_type_change(&self, differ: Pair<ColumnWalker<'_>>) -> Option<ColumnTypeChange> {
        // On MariaDB, JSON is an alias for LONGTEXT. https://mariadb.com/kb/en/json-data-type/
        if self.is_mariadb() {
            match (
                differ.previous.column_native_type(),
                differ.next.column_native_type(),
                differ.previous.column_type_family(),
                differ.next.column_type_family(),
            ) {
                (Some(MySqlType::LongText), Some(MySqlType::Json), _, _) => return None,
                (Some(_), Some(_), _, _) => (),
                (_, _, ColumnTypeFamily::String, ColumnTypeFamily::Json) => return None,
                _ => (),
            }
        }

        match (
            differ.previous.column_type_family_as_enum(),
            differ.next.column_type_family_as_enum(),
        ) {
            (Some(previous_enum), Some(next_enum)) => {
                if all_match(&mut previous_enum.values(), &mut next_enum.values()) {
                    return None;
                }

                return if previous_enum
                    .values()
                    .all(|previous_value| next_enum.values().any(|next_value| previous_value == next_value))
                {
                    Some(ColumnTypeChange::SafeCast)
                } else {
                    Some(ColumnTypeChange::RiskyCast)
                };
            }
            (Some(_), None) | (None, Some(_)) => return Some(ColumnTypeChange::RiskyCast),
            (None, None) => (),
        };

        if let Some(change) = differ
            .map(|walker| walker.column_native_type())
            .transpose()
            .and_then(native_type_change)
        {
            return Some(change);
        }

        None
    }

    fn index_should_be_renamed(&self, indexes: Pair<IndexWalker<'_>>) -> bool {
        // Implements correct comparison for truncated index names.
        let (previous_name, next_name) = indexes.as_ref().map(|idx| idx.name()).into_tuple();

        previous_name != next_name
    }

    fn lower_cases_table_names(&self) -> bool {
        self.lower_cases_table_names()
    }

    fn should_create_indexes_from_created_tables(&self) -> bool {
        false
    }

    fn should_ignore_json_defaults(&self) -> bool {
        true
    }

    fn should_skip_fk_indexes(&self) -> bool {
        true
    }

    fn table_names_match(&self, names: Pair<&str>) -> bool {
        if self.lower_cases_table_names() {
            names.previous.eq_ignore_ascii_case(names.next)
        } else {
            names.previous == names.next
        }
    }
}

fn risky() -> ColumnTypeChange {
    ColumnTypeChange::RiskyCast
}

fn not_castable() -> ColumnTypeChange {
    ColumnTypeChange::NotCastable
}

fn safe() -> ColumnTypeChange {
    ColumnTypeChange::SafeCast
}

fn native_type_change(types: Pair<&MySqlType>) -> Option<ColumnTypeChange> {
    let next = &types.next;

    Some(match &types.previous {
        MySqlType::BigInt => match next {
            MySqlType::BigInt => return None,

            MySqlType::Binary(_)
            | MySqlType::Bit(_)
            | MySqlType::Blob
            | MySqlType::Char(_)
            | MySqlType::Double
            | MySqlType::Float
            | MySqlType::LongText
            | MySqlType::LongBlob
            | MySqlType::MediumText
            | MySqlType::MediumBlob
            | MySqlType::Text
            | MySqlType::TinyBlob
            | MySqlType::Time(_)
            | MySqlType::TinyText
            | MySqlType::VarChar(_)
            | MySqlType::VarBinary(_) => safe(),

            MySqlType::Decimal(Some((n, m))) if n - m >= 20 => safe(),

            MySqlType::Int
            | MySqlType::MediumInt
            | MySqlType::SmallInt
            | MySqlType::TinyInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::UnsignedTinyInt
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedBigInt
            | MySqlType::Year => risky(),

            MySqlType::Decimal(_)
            | MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Timestamp(_)
            | MySqlType::Json => not_castable(),
        },
        MySqlType::Binary(size) => match next {
            MySqlType::Binary(n) if n == size => return None,
            MySqlType::Binary(n) if n > size => safe(),
            MySqlType::Binary(_) => risky(),

            MySqlType::Blob
            | MySqlType::Char(_)
            | MySqlType::Decimal(_)
            | MySqlType::Double
            | MySqlType::Float
            | MySqlType::LongBlob
            | MySqlType::LongText
            | MySqlType::MediumBlob
            | MySqlType::MediumInt
            | MySqlType::MediumText
            | MySqlType::Text
            | MySqlType::TinyBlob
            | MySqlType::TinyText
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => safe(),

            MySqlType::Bit(n) if *n >= size * 8 => safe(),

            MySqlType::BigInt
            | MySqlType::Bit(_)
            | MySqlType::Int
            | MySqlType::SmallInt
            | MySqlType::Time(_)
            | MySqlType::TinyInt
            | MySqlType::UnsignedBigInt
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::UnsignedTinyInt
            | MySqlType::Year => risky(),

            MySqlType::Date | MySqlType::DateTime(_) | MySqlType::Json | MySqlType::Timestamp(_) => not_castable(),
        },
        MySqlType::Bit(n) => match next {
            MySqlType::Bit(m) if n == m => return None,
            MySqlType::Bit(m) if n < m => safe(),
            MySqlType::Bit(_) => risky(),

            MySqlType::Blob
            | MySqlType::Char(_)
            | MySqlType::Int
            | MySqlType::LongBlob
            | MySqlType::LongText
            | MySqlType::MediumBlob
            | MySqlType::MediumInt
            | MySqlType::MediumText
            | MySqlType::SmallInt
            | MySqlType::Text
            | MySqlType::Year
            | MySqlType::TinyBlob
            | MySqlType::TinyInt
            | MySqlType::TinyText
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::UnsignedTinyInt
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => safe(),

            MySqlType::BigInt | MySqlType::UnsignedBigInt if *n == 64 => safe(),
            MySqlType::BigInt | MySqlType::UnsignedBigInt => risky(),

            MySqlType::Binary(m) if *n >= 8 * m => safe(),
            MySqlType::Binary(_) => not_castable(),

            MySqlType::Decimal(_) | MySqlType::Double | MySqlType::Float => risky(),

            MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Time(_)
            | MySqlType::Timestamp(_)
            | MySqlType::Json => not_castable(),
        },
        MySqlType::Blob => match next {
            MySqlType::Blob => return None,

            MySqlType::TinyBlob | MySqlType::LongBlob | MySqlType::MediumBlob => safe(),

            MySqlType::Binary(_)
            | MySqlType::Bit(_)
            | MySqlType::Char(_)
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => risky(),

            MySqlType::TinyInt
            | MySqlType::BigInt
            | MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Decimal(_)
            | MySqlType::Double
            | MySqlType::Float
            | MySqlType::Int
            | MySqlType::Json
            | MySqlType::MediumInt
            | MySqlType::SmallInt
            | MySqlType::Time(_)
            | MySqlType::Timestamp(_)
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::UnsignedTinyInt
            | MySqlType::UnsignedBigInt
            | MySqlType::Year => not_castable(),
        },
        MySqlType::Char(n) => match next {
            MySqlType::Char(m) if m == n => return None,
            MySqlType::VarChar(m) if m == n => safe(),
            MySqlType::VarChar(m) | MySqlType::Char(m) | MySqlType::VarBinary(m) | MySqlType::Binary(m) if m >= n => {
                safe()
            }
            MySqlType::VarChar(_) | MySqlType::Char(_) | MySqlType::Binary(_) | MySqlType::VarBinary(_) => risky(),

            // To string
            MySqlType::Blob
            | MySqlType::LongBlob
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::MediumBlob
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::TinyBlob => safe(),

            // Numeric types
            MySqlType::BigInt
            | MySqlType::Int
            | MySqlType::Decimal(_)
            | MySqlType::Float
            | MySqlType::Double
            | MySqlType::MediumInt
            | MySqlType::SmallInt
            | MySqlType::TinyInt
            | MySqlType::UnsignedBigInt
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::UnsignedTinyInt => risky(),

            MySqlType::Bit(_)
            | MySqlType::Json
            | MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Timestamp(_)
            | MySqlType::Time(_)
            | MySqlType::Year => risky(),
        },
        MySqlType::Date => match next {
            MySqlType::Date => return None,

            MySqlType::DateTime(_)
            | MySqlType::BigInt
            | MySqlType::UnsignedBigInt
            | MySqlType::Int
            | MySqlType::UnsignedInt => safe(),

            MySqlType::Decimal(_) | MySqlType::Float | MySqlType::Double => safe(),

            MySqlType::TinyInt
            | MySqlType::UnsignedTinyInt
            | MySqlType::Year
            | MySqlType::SmallInt
            | MySqlType::MediumInt
            | MySqlType::Time(_)
            | MySqlType::Timestamp(_)
            | MySqlType::Json
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt => not_castable(),

            // To string
            MySqlType::Binary(_)
            | MySqlType::Bit(_)
            | MySqlType::Char(_)
            | MySqlType::LongText
            | MySqlType::TinyBlob
            | MySqlType::MediumBlob
            | MySqlType::Blob
            | MySqlType::LongBlob
            | MySqlType::MediumText
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => safe(),
        },
        MySqlType::DateTime(n) => match next {
            MySqlType::DateTime(m) if n < m => safe(),
            MySqlType::DateTime(m) if n > m => risky(),
            MySqlType::DateTime(_) => return None,

            // To string
            MySqlType::Binary(_)
            | MySqlType::Char(_)
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::Blob
            | MySqlType::TinyBlob
            | MySqlType::LongBlob
            | MySqlType::MediumBlob
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => safe(),

            // Numeric types
            MySqlType::BigInt | MySqlType::UnsignedBigInt | MySqlType::Bit(64) => safe(),
            MySqlType::Bit(_)
            | MySqlType::Double
            | MySqlType::Float
            | MySqlType::UnsignedTinyInt
            | MySqlType::Decimal(_)
            | MySqlType::TinyInt
            | MySqlType::Int
            | MySqlType::Json
            | MySqlType::UnsignedInt
            | MySqlType::SmallInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::MediumInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::Year => not_castable(),

            MySqlType::Timestamp(_) | MySqlType::Time(_) | MySqlType::Date => safe(),
        },
        MySqlType::Decimal(n) => match next {
            MySqlType::Decimal(m) if n == m => return None,
            MySqlType::Decimal(_) => risky(),

            // To string
            MySqlType::Binary(_)
            | MySqlType::Char(_)
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::Blob
            | MySqlType::TinyBlob
            | MySqlType::LongBlob
            | MySqlType::MediumBlob
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => safe(),

            // Numeric
            MySqlType::BigInt
            | MySqlType::Bit(_)
            | MySqlType::Time(_)
            | MySqlType::Int
            | MySqlType::MediumInt
            | MySqlType::SmallInt
            | MySqlType::TinyInt
            | MySqlType::UnsignedBigInt
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::UnsignedTinyInt
            | MySqlType::Float
            | MySqlType::Double
            | MySqlType::Year
            | MySqlType::Json => risky(),

            MySqlType::DateTime(_) | MySqlType::Timestamp(_) | MySqlType::Date => not_castable(),
        },
        MySqlType::Double => match next {
            MySqlType::Double => return None,

            MySqlType::Float => safe(),
            MySqlType::Bit(64) => safe(),
            MySqlType::Bit(_) => not_castable(),

            // Integer types
            MySqlType::UnsignedTinyInt
            | MySqlType::Decimal(_)
            | MySqlType::BigInt
            | MySqlType::UnsignedBigInt
            | MySqlType::TinyInt
            | MySqlType::Int
            | MySqlType::Json
            | MySqlType::UnsignedInt
            | MySqlType::SmallInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::MediumInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::Year => safe(),

            MySqlType::Binary(n) | MySqlType::Char(n) | MySqlType::VarBinary(n) | MySqlType::VarChar(n) => {
                if *n >= 32 {
                    risky()
                } else {
                    not_castable()
                }
            }

            // To string
            MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::Blob
            | MySqlType::TinyBlob
            | MySqlType::LongBlob
            | MySqlType::MediumBlob => safe(),

            MySqlType::Time(_) => safe(),

            MySqlType::Timestamp(_) | MySqlType::DateTime(_) | MySqlType::Date => not_castable(),
        },
        MySqlType::Float => match next {
            MySqlType::Float => return None,

            MySqlType::Double => safe(),

            MySqlType::Bit(n) if *n >= 32 => safe(),
            MySqlType::Bit(_) => not_castable(),

            // Integer types
            MySqlType::UnsignedTinyInt
            | MySqlType::Decimal(_)
            | MySqlType::BigInt
            | MySqlType::UnsignedBigInt
            | MySqlType::TinyInt
            | MySqlType::Int
            | MySqlType::Json
            | MySqlType::UnsignedInt
            | MySqlType::SmallInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::MediumInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::Year => safe(),

            MySqlType::Binary(n) | MySqlType::Char(n) | MySqlType::VarBinary(n) | MySqlType::VarChar(n) => {
                if *n >= 32 {
                    risky()
                } else {
                    not_castable()
                }
            }

            // To string
            MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::Blob
            | MySqlType::TinyBlob
            | MySqlType::LongBlob
            | MySqlType::MediumBlob => safe(),

            MySqlType::Time(_) => safe(),

            MySqlType::Timestamp(_) | MySqlType::DateTime(_) | MySqlType::Date => not_castable(),
        },
        MySqlType::Int => match next {
            MySqlType::Int => return None,

            MySqlType::BigInt
            | MySqlType::Binary(_)
            | MySqlType::VarBinary(_)
            | MySqlType::Char(_)
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::Text
            | MySqlType::TinyBlob
            | MySqlType::Blob
            | MySqlType::MediumBlob
            | MySqlType::LongBlob
            | MySqlType::TinyText
            | MySqlType::VarChar(_) => safe(),

            MySqlType::Bit(n) if *n >= 32 => safe(),
            MySqlType::Bit(_) => risky(),

            MySqlType::TinyInt
            | MySqlType::SmallInt
            | MySqlType::MediumInt
            | MySqlType::Year
            | MySqlType::Float
            | MySqlType::Double
            | MySqlType::Decimal(_) => risky(),

            // Signed to unsigned
            MySqlType::UnsignedBigInt
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedTinyInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt => risky(),

            MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Timestamp(_)
            | MySqlType::Time(_)
            | MySqlType::Json => not_castable(),
        },
        MySqlType::Json => match next {
            MySqlType::Json => return None,

            // To string
            MySqlType::Binary(_)
            | MySqlType::Char(_)
            | MySqlType::TinyBlob
            | MySqlType::Blob
            | MySqlType::MediumBlob
            | MySqlType::LongBlob
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => safe(),

            MySqlType::Time(_)
            | MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Year
            | MySqlType::Timestamp(_) => not_castable(),

            // Numeric
            MySqlType::BigInt
            | MySqlType::Bit(_)
            | MySqlType::Decimal(_)
            | MySqlType::Int
            | MySqlType::MediumInt
            | MySqlType::SmallInt
            | MySqlType::TinyInt
            | MySqlType::UnsignedBigInt
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::UnsignedTinyInt
            | MySqlType::Float
            | MySqlType::Double => not_castable(),
        },
        MySqlType::LongBlob => match next {
            MySqlType::LongBlob => return None,

            MySqlType::TinyBlob | MySqlType::Blob | MySqlType::MediumBlob => safe(),

            MySqlType::Binary(_)
            | MySqlType::Bit(_)
            | MySqlType::Char(_)
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => risky(),

            MySqlType::TinyInt
            | MySqlType::BigInt
            | MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Decimal(_)
            | MySqlType::Double
            | MySqlType::Float
            | MySqlType::Int
            | MySqlType::Json
            | MySqlType::MediumInt
            | MySqlType::SmallInt
            | MySqlType::Time(_)
            | MySqlType::Timestamp(_)
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::UnsignedTinyInt
            | MySqlType::UnsignedBigInt
            | MySqlType::Year => not_castable(),
        },
        MySqlType::LongText => match next {
            MySqlType::LongText => return None,

            // Numeric types
            MySqlType::BigInt
            | MySqlType::Int
            | MySqlType::Decimal(_)
            | MySqlType::Float
            | MySqlType::Double
            | MySqlType::MediumInt
            | MySqlType::SmallInt
            | MySqlType::TinyInt
            | MySqlType::UnsignedBigInt
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::UnsignedTinyInt => risky(),

            // To string
            MySqlType::Binary(_)
            | MySqlType::Bit(_)
            | MySqlType::Char(_)
            | MySqlType::Blob
            | MySqlType::MediumBlob
            | MySqlType::Text
            | MySqlType::LongBlob
            | MySqlType::MediumText
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => safe(),

            MySqlType::TinyText | MySqlType::TinyBlob => risky(),

            MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Timestamp(_)
            | MySqlType::Json
            | MySqlType::Time(_)
            | MySqlType::Year => risky(),
        },
        MySqlType::MediumBlob => match next {
            MySqlType::MediumBlob => return None,

            MySqlType::TinyBlob | MySqlType::Blob | MySqlType::LongBlob => safe(),

            MySqlType::Binary(_)
            | MySqlType::Bit(_)
            | MySqlType::Char(_)
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => risky(),

            MySqlType::TinyInt
            | MySqlType::BigInt
            | MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Decimal(_)
            | MySqlType::Double
            | MySqlType::Float
            | MySqlType::Int
            | MySqlType::Json
            | MySqlType::MediumInt
            | MySqlType::SmallInt
            | MySqlType::Time(_)
            | MySqlType::Timestamp(_)
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::UnsignedTinyInt
            | MySqlType::UnsignedBigInt
            | MySqlType::Year => not_castable(),
        },
        MySqlType::MediumInt => match next {
            MySqlType::MediumInt => return None,

            MySqlType::BigInt
            | MySqlType::Binary(_)
            | MySqlType::VarBinary(_)
            | MySqlType::Char(_)
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::Text
            | MySqlType::TinyBlob
            | MySqlType::Blob
            | MySqlType::MediumBlob
            | MySqlType::LongBlob
            | MySqlType::TinyText
            | MySqlType::VarChar(_) => safe(),

            MySqlType::Bit(n) if *n >= 32 => safe(),
            MySqlType::Bit(_) => risky(),

            MySqlType::TinyInt
            | MySqlType::SmallInt
            | MySqlType::Int
            | MySqlType::Year
            | MySqlType::Float
            | MySqlType::Double
            | MySqlType::Decimal(_) => risky(),

            // Signed to unsigned
            MySqlType::UnsignedBigInt
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedTinyInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt => risky(),

            MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Timestamp(_)
            | MySqlType::Time(_)
            | MySqlType::Json => not_castable(),
        },
        MySqlType::MediumText => match next {
            MySqlType::MediumText => return None,

            // Numeric types
            MySqlType::BigInt
            | MySqlType::Int
            | MySqlType::Decimal(_)
            | MySqlType::Float
            | MySqlType::Double
            | MySqlType::MediumInt
            | MySqlType::SmallInt
            | MySqlType::TinyInt
            | MySqlType::UnsignedBigInt
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::UnsignedTinyInt => risky(),

            // To string
            MySqlType::Binary(_)
            | MySqlType::Bit(_)
            | MySqlType::Char(_)
            | MySqlType::Blob
            | MySqlType::MediumBlob
            | MySqlType::Text
            | MySqlType::LongBlob
            | MySqlType::LongText
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => safe(),

            MySqlType::TinyText | MySqlType::TinyBlob => risky(),

            MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Timestamp(_)
            | MySqlType::Json
            | MySqlType::Time(_)
            | MySqlType::Year => risky(),
        },

        MySqlType::SmallInt => match next {
            MySqlType::SmallInt => return None,

            // To string
            MySqlType::Binary(_)
            | MySqlType::Bit(_)
            | MySqlType::Char(_)
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::Blob
            | MySqlType::LongBlob
            | MySqlType::MediumBlob
            | MySqlType::TinyBlob
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => risky(),

            // Larger int types
            MySqlType::MediumInt | MySqlType::Int | MySqlType::BigInt | MySqlType::Time(_) => safe(),

            MySqlType::TinyInt | MySqlType::Float | MySqlType::Double | MySqlType::Year => risky(),

            // Signed to unsigned
            MySqlType::UnsignedBigInt
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedTinyInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt => risky(),

            MySqlType::Decimal(_)
            | MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Timestamp(_)
            | MySqlType::Json => not_castable(),
        },

        MySqlType::Text => match next {
            MySqlType::Text => return None,

            // Numeric types
            MySqlType::BigInt
            | MySqlType::Int
            | MySqlType::Decimal(_)
            | MySqlType::Float
            | MySqlType::Double
            | MySqlType::MediumInt
            | MySqlType::SmallInt
            | MySqlType::TinyInt
            | MySqlType::UnsignedBigInt
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::UnsignedTinyInt => risky(),

            // To string
            MySqlType::Binary(_)
            | MySqlType::Bit(_)
            | MySqlType::Char(_)
            | MySqlType::Blob
            | MySqlType::MediumBlob
            | MySqlType::MediumText
            | MySqlType::LongBlob
            | MySqlType::LongText
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => safe(),

            MySqlType::TinyText | MySqlType::TinyBlob => risky(),

            MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Timestamp(_)
            | MySqlType::Json
            | MySqlType::Time(_)
            | MySqlType::Year => risky(),
        },

        MySqlType::Time(n) => match next {
            MySqlType::Time(None) if n.unwrap_or(0) == 0 => return None,
            MySqlType::Time(m) if n == m => return None,
            MySqlType::Time(_) => safe(),

            // To string
            MySqlType::Binary(_)
            | MySqlType::Char(_)
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::Blob
            | MySqlType::Decimal(_)
            | MySqlType::TinyBlob
            | MySqlType::LongBlob
            | MySqlType::MediumBlob
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => safe(),

            MySqlType::Date | MySqlType::DateTime(_) | MySqlType::Timestamp(_) => risky(),

            MySqlType::Json | MySqlType::Year => not_castable(),

            // To numeric
            MySqlType::BigInt
            | MySqlType::Bit(_)
            | MySqlType::Int
            | MySqlType::Float
            | MySqlType::Double
            | MySqlType::TinyInt
            | MySqlType::UnsignedTinyInt
            | MySqlType::MediumInt
            | MySqlType::SmallInt
            | MySqlType::UnsignedBigInt
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt => safe(),
        },
        MySqlType::Timestamp(n) => match next {
            MySqlType::Timestamp(m) if n < m => safe(),
            MySqlType::Timestamp(m) if n > m => risky(),
            MySqlType::Timestamp(_) => return None,

            // To string
            MySqlType::Binary(_)
            | MySqlType::Char(_)
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::Blob
            | MySqlType::TinyBlob
            | MySqlType::LongBlob
            | MySqlType::MediumBlob
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => safe(),

            // Numeric types
            MySqlType::BigInt | MySqlType::UnsignedBigInt | MySqlType::Bit(64) => safe(),
            MySqlType::Bit(_)
            | MySqlType::Double
            | MySqlType::Float
            | MySqlType::UnsignedTinyInt
            | MySqlType::Decimal(_)
            | MySqlType::TinyInt
            | MySqlType::Int
            | MySqlType::Json
            | MySqlType::UnsignedInt
            | MySqlType::SmallInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::MediumInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::Year => not_castable(),

            MySqlType::DateTime(_) | MySqlType::Time(_) | MySqlType::Date => safe(),
        },

        MySqlType::TinyBlob => match next {
            MySqlType::TinyBlob => return None,

            MySqlType::LongBlob | MySqlType::Blob | MySqlType::MediumBlob => safe(),

            MySqlType::Binary(_)
            | MySqlType::Bit(_)
            | MySqlType::Char(_)
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => risky(),

            MySqlType::TinyInt
            | MySqlType::BigInt
            | MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Decimal(_)
            | MySqlType::Double
            | MySqlType::Float
            | MySqlType::Int
            | MySqlType::Json
            | MySqlType::MediumInt
            | MySqlType::SmallInt
            | MySqlType::Time(_)
            | MySqlType::Timestamp(_)
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::UnsignedTinyInt
            | MySqlType::UnsignedBigInt
            | MySqlType::Year => not_castable(),
        },

        MySqlType::TinyInt => match next {
            MySqlType::TinyInt => return None,

            // To string
            MySqlType::Binary(_)
            | MySqlType::Bit(_)
            | MySqlType::Char(_)
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::Blob
            | MySqlType::LongBlob
            | MySqlType::MediumBlob
            | MySqlType::TinyBlob
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => risky(),

            // Larger int types
            MySqlType::SmallInt | MySqlType::MediumInt | MySqlType::Int | MySqlType::BigInt | MySqlType::Time(_) => {
                safe()
            }

            MySqlType::Float | MySqlType::Double | MySqlType::Year => risky(),

            // Signed to unsigned
            MySqlType::UnsignedBigInt
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedTinyInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt => risky(),

            MySqlType::Decimal(_)
            | MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Timestamp(_)
            | MySqlType::Json => not_castable(),
        },

        MySqlType::TinyText => match next {
            MySqlType::TinyText => return None,

            // Numeric types
            MySqlType::BigInt
            | MySqlType::Int
            | MySqlType::Decimal(_)
            | MySqlType::Float
            | MySqlType::Double
            | MySqlType::MediumInt
            | MySqlType::SmallInt
            | MySqlType::TinyInt
            | MySqlType::UnsignedBigInt
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::UnsignedTinyInt => risky(),

            // To string
            MySqlType::Binary(_)
            | MySqlType::Bit(_)
            | MySqlType::Char(_)
            | MySqlType::Blob
            | MySqlType::MediumBlob
            | MySqlType::MediumText
            | MySqlType::LongBlob
            | MySqlType::LongText
            | MySqlType::Text
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => safe(),

            MySqlType::TinyBlob => risky(),

            MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Timestamp(_)
            | MySqlType::Json
            | MySqlType::Time(_)
            | MySqlType::Year => risky(),
        },

        MySqlType::UnsignedBigInt => match next {
            MySqlType::UnsignedBigInt => return None,

            // To string
            MySqlType::Binary(_)
            | MySqlType::Char(_)
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::Blob
            | MySqlType::LongBlob
            | MySqlType::MediumBlob
            | MySqlType::TinyBlob
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => risky(),

            // Numeric types
            MySqlType::BigInt
            | MySqlType::Bit(_)
            | MySqlType::Int
            | MySqlType::Float
            | MySqlType::Double
            | MySqlType::MediumInt
            | MySqlType::SmallInt
            | MySqlType::TinyInt
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::UnsignedTinyInt => risky(),

            MySqlType::Time(_) | MySqlType::Year => risky(),

            MySqlType::Decimal(_)
            | MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Timestamp(_)
            | MySqlType::Json => not_castable(),
        },

        MySqlType::UnsignedInt => match next {
            MySqlType::UnsignedInt => return None,

            // To string
            MySqlType::Binary(_)
            | MySqlType::Char(_)
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::Blob
            | MySqlType::LongBlob
            | MySqlType::MediumBlob
            | MySqlType::TinyBlob
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => risky(),

            // Numeric types
            MySqlType::BigInt
            | MySqlType::Bit(_)
            | MySqlType::Int
            | MySqlType::Float
            | MySqlType::Double
            | MySqlType::MediumInt
            | MySqlType::SmallInt
            | MySqlType::TinyInt
            | MySqlType::UnsignedBigInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::UnsignedTinyInt => risky(),

            MySqlType::Time(_) | MySqlType::Year => risky(),

            MySqlType::Decimal(_)
            | MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Timestamp(_)
            | MySqlType::Json => not_castable(),
        },
        MySqlType::UnsignedMediumInt => match next {
            MySqlType::UnsignedMediumInt => return None,

            // To string
            MySqlType::Binary(_)
            | MySqlType::Char(_)
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::Blob
            | MySqlType::LongBlob
            | MySqlType::MediumBlob
            | MySqlType::TinyBlob
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => risky(),

            // Numeric types
            MySqlType::BigInt
            | MySqlType::Bit(_)
            | MySqlType::Int
            | MySqlType::Float
            | MySqlType::Double
            | MySqlType::MediumInt
            | MySqlType::SmallInt
            | MySqlType::TinyInt
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedBigInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::UnsignedTinyInt => risky(),

            MySqlType::Time(_) | MySqlType::Year => risky(),

            MySqlType::Decimal(_)
            | MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Timestamp(_)
            | MySqlType::Json => not_castable(),
        },
        MySqlType::UnsignedSmallInt => match next {
            MySqlType::UnsignedSmallInt => return None,

            // To string
            MySqlType::Binary(_)
            | MySqlType::Char(_)
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::Blob
            | MySqlType::LongBlob
            | MySqlType::MediumBlob
            | MySqlType::TinyBlob
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => risky(),

            MySqlType::UnsignedInt | MySqlType::UnsignedMediumInt | MySqlType::UnsignedBigInt => safe(),

            // Numeric types
            MySqlType::BigInt
            | MySqlType::Bit(_)
            | MySqlType::Int
            | MySqlType::Float
            | MySqlType::Double
            | MySqlType::MediumInt
            | MySqlType::SmallInt
            | MySqlType::TinyInt
            | MySqlType::UnsignedTinyInt => risky(),

            MySqlType::Time(_) | MySqlType::Year => risky(),

            MySqlType::Decimal(_)
            | MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Timestamp(_)
            | MySqlType::Json => not_castable(),
        },
        MySqlType::UnsignedTinyInt => match next {
            MySqlType::UnsignedTinyInt => return None,

            // To string
            MySqlType::Binary(_)
            | MySqlType::Char(_)
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::Blob
            | MySqlType::LongBlob
            | MySqlType::MediumBlob
            | MySqlType::TinyBlob
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => risky(),

            MySqlType::UnsignedInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedBigInt
            | MySqlType::UnsignedSmallInt => safe(),

            // Numeric types
            MySqlType::BigInt
            | MySqlType::Bit(_)
            | MySqlType::Int
            | MySqlType::Float
            | MySqlType::Double
            | MySqlType::MediumInt
            | MySqlType::SmallInt
            | MySqlType::TinyInt => risky(),

            MySqlType::Time(_) | MySqlType::Year => risky(),

            MySqlType::Decimal(_)
            | MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Timestamp(_)
            | MySqlType::Json => not_castable(),
        },
        MySqlType::VarBinary(n) => match next {
            MySqlType::VarBinary(m) if n > m => risky(),
            MySqlType::VarBinary(m) if n < m => safe(),
            MySqlType::VarBinary(_) => return None,

            MySqlType::LongBlob | MySqlType::Blob | MySqlType::MediumBlob => safe(),
            MySqlType::Binary(m) if m > n => safe(),

            MySqlType::Binary(_)
            | MySqlType::Bit(_)
            | MySqlType::Char(_)
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::TinyBlob
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::VarChar(_) => risky(),

            MySqlType::TinyInt
            | MySqlType::BigInt
            | MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Decimal(_)
            | MySqlType::Double
            | MySqlType::Float
            | MySqlType::Int
            | MySqlType::Json
            | MySqlType::MediumInt
            | MySqlType::SmallInt
            | MySqlType::Time(_)
            | MySqlType::Timestamp(_)
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::UnsignedTinyInt
            | MySqlType::UnsignedBigInt
            | MySqlType::Year => not_castable(),
        },
        MySqlType::VarChar(n) => match next {
            MySqlType::VarChar(m) if m == n => return None,
            MySqlType::Char(m) if m == n => safe(),
            MySqlType::VarChar(m) | MySqlType::Char(m) | MySqlType::VarBinary(m) | MySqlType::Binary(m) if m >= n => {
                safe()
            }
            MySqlType::VarChar(_) | MySqlType::Char(_) | MySqlType::Binary(_) | MySqlType::VarBinary(_) => risky(),

            // To string
            MySqlType::Blob
            | MySqlType::LongBlob
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::MediumBlob
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::TinyBlob => safe(),

            // Numeric types
            MySqlType::BigInt
            | MySqlType::Bit(_)
            | MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Decimal(_)
            | MySqlType::Double
            | MySqlType::Float
            | MySqlType::Int
            | MySqlType::MediumInt
            | MySqlType::SmallInt
            | MySqlType::Time(_)
            | MySqlType::Timestamp(_)
            | MySqlType::TinyInt
            | MySqlType::UnsignedBigInt
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt
            | MySqlType::UnsignedTinyInt
            | MySqlType::Year
            | MySqlType::Json => risky(),
        },
        MySqlType::Year => match next {
            MySqlType::Year => return None,

            // To string
            MySqlType::Binary(_)
            | MySqlType::Blob
            | MySqlType::Char(_)
            | MySqlType::LongBlob
            | MySqlType::LongText
            | MySqlType::MediumText
            | MySqlType::MediumBlob
            | MySqlType::Text
            | MySqlType::TinyText
            | MySqlType::TinyBlob
            | MySqlType::VarBinary(_)
            | MySqlType::VarChar(_) => safe(),

            // To Integer
            MySqlType::BigInt
            | MySqlType::Bit(_)
            | MySqlType::Int
            | MySqlType::MediumInt
            | MySqlType::SmallInt
            | MySqlType::UnsignedBigInt
            | MySqlType::UnsignedInt
            | MySqlType::UnsignedMediumInt
            | MySqlType::UnsignedSmallInt => safe(),

            MySqlType::Float | MySqlType::Double => safe(),

            MySqlType::Decimal(_) | MySqlType::Json => risky(),

            MySqlType::Date
            | MySqlType::DateTime(_)
            | MySqlType::Time(_)
            | MySqlType::TinyInt
            | MySqlType::Timestamp(_)
            | MySqlType::UnsignedTinyInt => not_castable(), // out of range
        },
    })
}
