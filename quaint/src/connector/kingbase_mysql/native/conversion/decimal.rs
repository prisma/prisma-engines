use bytes::BytesMut;
use kingbase_postgres_types::{FromSql, IsNull, ToSql, Type};

/// Adapter between Quaint's BigDecimal 0.3 value type and the Kingbase
/// driver's BigDecimal 0.4 wire codec.
#[derive(Debug, Clone)]
pub struct DecimalWrapper(pub bigdecimal::BigDecimal);

impl FromSql<'_> for DecimalWrapper {
    fn from_sql(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let value = <kingbase_bigdecimal::BigDecimal as FromSql>::from_sql(ty, raw)?;
        let value =
            bigdecimal::BigDecimal::parse_bytes(value.to_string().as_bytes(), 10).ok_or("invalid NUMERIC value")?;

        Ok(Self(value))
    }

    fn accepts(ty: &Type) -> bool {
        ty == &Type::NUMERIC
    }
}

impl ToSql for DecimalWrapper {
    fn to_sql(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        let value = kingbase_bigdecimal::BigDecimal::parse_bytes(self.0.to_string().as_bytes(), 10)
            .ok_or("invalid NUMERIC value")?;

        <kingbase_bigdecimal::BigDecimal as ToSql>::to_sql(&value, ty, out)
    }

    fn accepts(ty: &Type) -> bool {
        ty == &Type::NUMERIC
    }

    kingbase_postgres_types::to_sql_checked!();
}
