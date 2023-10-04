use super::*;
use crate::bigdecimal::BigDecimal;

test_type!(decimal(
    postgresql,
    "decimal(10,2)",
    Value::null_numeric(),
    Value::numeric(BigDecimal::from_str("3.14")?)
));

test_type!(decimal_10_2(
    postgresql,
    "decimal(10, 2)",
    (
        Value::numeric(BigDecimal::from_str("3950.123456")?),
        Value::numeric(BigDecimal::from_str("3950.12")?)
    )
));

test_type!(decimal_35_6(
    postgresql,
    "decimal(35, 6)",
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

test_type!(decimal_35_2(
    postgresql,
    "decimal(35, 2)",
    (
        Value::numeric(BigDecimal::from_str("3950.123456")?),
        Value::numeric(BigDecimal::from_str("3950.12")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("3950.1256")?),
        Value::numeric(BigDecimal::from_str("3950.13")?)
    ),
));

test_type!(decimal_4_0(
    postgresql,
    "decimal(4, 0)",
    Value::numeric(BigDecimal::from_str("3950")?)
));

test_type!(decimal_65_30(
    postgresql,
    "decimal(65, 30)",
    (
        Value::numeric(BigDecimal::from_str("1.2")?),
        Value::numeric(BigDecimal::from_str("1.2000000000000000000000000000")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("3.141592653589793238462643383279")?),
        Value::numeric(BigDecimal::from_str("3.141592653589793238462643383279")?)
    )
));

test_type!(decimal_65_34(
    postgresql,
    "decimal(65, 34)",
    (
        Value::numeric(BigDecimal::from_str("3.1415926535897932384626433832795028")?),
        Value::numeric(BigDecimal::from_str("3.1415926535897932384626433832795028")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("1.23456789012345678901234567895")?),
        Value::numeric(BigDecimal::from_str("1.23456789012345678901234567895")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("1.234567890123456789012345678949999")?),
        Value::numeric(BigDecimal::from_str("1.234567890123456789012345678949999")?)
    ),
));

test_type!(decimal_35_0(
    postgresql,
    "decimal(35, 0)",
    Value::numeric(BigDecimal::from_str("79228162514264337593543950335")?),
));

test_type!(decimal_35_1(
    postgresql,
    "decimal(35, 1)",
    (
        Value::numeric(BigDecimal::from_str("79228162514264337593543950335")?),
        Value::numeric(BigDecimal::from_str("79228162514264337593543950335.0")?)
    ),
    (
        Value::numeric(BigDecimal::from_str("4951760157141521099596496896")?),
        Value::numeric(BigDecimal::from_str("4951760157141521099596496896.0")?)
    )
));

test_type!(decimal_128_6(
    postgresql,
    "decimal(128, 6)",
    Value::numeric(BigDecimal::from_str(
        "93431006223456789876545678909876545678903434369343100622345678987654567890987654567890343436999999100622345678343699999910.345678"
    )?),
));

test_type!(decimal_array(
    postgresql,
    "decimal(10,2)[]",
    Value::null_array(),
    Value::array(vec![BigDecimal::from_str("3.14")?, BigDecimal::from_str("5.12")?])
));

test_type!(money(
    postgresql,
    "money",
    Value::null_numeric(),
    Value::numeric(BigDecimal::from_str("1.12")?)
));

test_type!(money_array(
    postgresql,
    "money[]",
    Value::null_array(),
    Value::array(vec![BigDecimal::from_str("1.12")?, BigDecimal::from_str("1.12")?])
));

test_type!(float4(
    postgresql,
    "float4",
    (Value::null_numeric(), Value::null_float()),
    (
        Value::numeric(BigDecimal::from_str("1.123456")?),
        Value::float(1.123456)
    )
));

test_type!(float8(
    postgresql,
    "float8",
    (Value::null_numeric(), Value::null_double()),
    (
        Value::numeric(BigDecimal::from_str("1.123456")?),
        Value::double(1.123456)
    )
));
