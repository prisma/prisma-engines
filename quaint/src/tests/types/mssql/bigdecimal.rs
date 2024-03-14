use super::*;
use crate::bigdecimal::BigDecimal;
use std::str::FromStr;

test_type!(numeric(
    mssql,
    "numeric(10,2)",
    Value::null_numeric(),
    Value::numeric(BigDecimal::from_str("3.14")?)
));

test_type!(numeric_10_2(
    mssql,
    "numeric(10,2)",
    (
        Value::numeric(BigDecimal::from_str("3950.123456")?),
        Value::numeric(BigDecimal::from_str("3950.12")?)
    )
));

test_type!(numeric_35_6(
    mssql,
    "numeric(35, 6)",
    (
        Value::numeric(BigDecimal::from_str("3950")?),
        Value::numeric(BigDecimal::from_str("3950.000000")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("3950.123456")?),
        Value::numeric(BigDecimal::from_str("3950.123456")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("0.1")?),
        Value::numeric(BigDecimal::from_str("0.100000")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("0.01")?),
        Value::numeric(BigDecimal::from_str("0.010000")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("0.001")?),
        Value::numeric(BigDecimal::from_str("0.001000")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("0.0001")?),
        Value::numeric(BigDecimal::from_str("0.000100")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("0.00001")?),
        Value::numeric(BigDecimal::from_str("0.000010")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("0.000001")?),
        Value::numeric(BigDecimal::from_str("0.000001")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("1")?),
        Value::numeric(BigDecimal::from_str("1.000000")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("-100")?),
        Value::numeric(BigDecimal::from_str("-100.000000")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("-123.456")?),
        Value::numeric(BigDecimal::from_str("-123.456000")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("119996.25")?),
        Value::numeric(BigDecimal::from_str("119996.250000")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("1000000")?),
        Value::numeric(BigDecimal::from_str("1000000.000000")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("9999999.99999")?),
        Value::numeric(BigDecimal::from_str("9999999.999990")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("12340.56789")?),
        Value::numeric(BigDecimal::from_str("12340.567890")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("18446744073709551615")?),
        Value::numeric(BigDecimal::from_str("18446744073709551615.000000")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("-18446744073709551615")?),
        Value::numeric(BigDecimal::from_str("-18446744073709551615.000000")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("0.10001")?),
        Value::numeric(BigDecimal::from_str("0.100010")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("0.12345")?),
        Value::numeric(BigDecimal::from_str("0.123450")?)
    ),
));

test_type!(numeric_35_2(
    mssql,
    "numeric(35, 2)",
    (
        Value::numeric(BigDecimal::from_str("3950.123456")?),
        Value::numeric(BigDecimal::from_str("3950.12")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("3950.1256")?),
        Value::numeric(BigDecimal::from_str("3950.13")?)
    ),
));

test_type!(numeric_4_0(
    mssql,
    "numeric(4, 0)",
    Value::numeric(BigDecimal::from_str("3950")?)
));

test_type!(numeric_35_0(
    mssql,
    "numeric(35, 0)",
    Value::numeric(BigDecimal::from_str("79228162514264337593543950335")?),
));

test_type!(numeric_35_1(
    mssql,
    "numeric(35, 1)",
    (
        Value::numeric(BigDecimal::from_str("79228162514264337593543950335")?),
        Value::numeric(BigDecimal::from_str("79228162514264337593543950335.0")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("4951760157141521099596496896")?),
        Value::numeric(BigDecimal::from_str("4951760157141521099596496896.0")?)
    )
));

// Highest mantissa supported on SQL Server
test_type!(numeric_38_6(
    mssql,
    "numeric(38, 6)",
    Value::numeric(BigDecimal::from_str("9343234567898765456789043634999.345678")?),
));

test_type!(money(
    mssql,
    "money",
    (Value::null_numeric(), Value::null_double()),
    (Value::numeric(BigDecimal::from_str("3.14")?), Value::double(3.14))
));

test_type!(smallmoney(
    mssql,
    "smallmoney",
    (Value::null_numeric(), Value::null_double()),
    (Value::numeric(BigDecimal::from_str("3.14")?), Value::double(3.14))
));

test_type!(float_24(
    mssql,
    "float(24)",
    (Value::null_numeric(), Value::null_float()),
    (
        Value::numeric(BigDecimal::from_str("1.123456")?),
        Value::float(1.123456)
    )
));

test_type!(real(
    mssql,
    "real",
    (Value::null_numeric(), Value::null_float()),
    (
        Value::numeric(BigDecimal::from_str("1.123456")?),
        Value::float(1.123456)
    )
));

test_type!(float_53(
    mssql,
    "float(53)",
    (Value::null_numeric(), Value::null_double()),
    (
        Value::numeric(BigDecimal::from_str("1.123456789012345")?),
        Value::double(1.123456789012345)
    )
));
