//! We implement the postgres conversions here to fix a disastrous dependency hell.

use byteorder::{BigEndian, ReadBytesExt};
use bytes::{BufMut, BytesMut};
use postgres_types::{to_sql_checked, FromSql, IsNull, ToSql, Type};
use rust_decimal::{prelude::Zero, Decimal};
use std::convert::TryInto;
use std::{error, fmt, io::Cursor};

const MAX_PRECISION: u32 = 28;

#[derive(Debug, Clone)]
pub struct DecimalWrapper(pub Decimal);

#[derive(Debug, Clone)]
pub struct InvalidDecimal {
    inner: Option<String>,
}

impl fmt::Display for InvalidDecimal {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref msg) = self.inner {
            fmt.write_fmt(format_args!("Invalid Decimal: {}", msg))
        } else {
            fmt.write_str("Invalid Decimal")
        }
    }
}

impl error::Error for InvalidDecimal {}

struct PostgresDecimal<D> {
    neg: bool,
    weight: i16,
    scale: u16,
    digits: D,
}

fn from_postgres<D: ExactSizeIterator<Item = u16>>(dec: PostgresDecimal<D>) -> Result<Decimal, InvalidDecimal> {
    let PostgresDecimal {
        neg,
        scale,
        digits,
        weight,
    } = dec;

    let mut digits = digits.into_iter().collect::<Vec<_>>();

    let fractionals_part_count = digits.len() as i32 + (-weight as i32) - 1;
    let integers_part_count = weight as i32 + 1;

    let mut result = Decimal::zero();

    // adding integer part
    if integers_part_count > 0 {
        let (start_integers, last) = if integers_part_count > digits.len() as i32 {
            (integers_part_count - digits.len() as i32, digits.len() as i32)
        } else {
            (0, integers_part_count)
        };

        let integers: Vec<_> = digits.drain(..last as usize).collect();

        for digit in integers {
            result *= Decimal::from_i128_with_scale(10i128.pow(4), 0);
            result += Decimal::new(digit as i64, 0);
        }

        result *= Decimal::from_i128_with_scale(10i128.pow(4 * start_integers as u32), 0);
    }

    // adding fractional part
    if fractionals_part_count > 0 {
        let dec: Vec<_> = digits.into_iter().collect();
        let start_fractionals = if weight < 0 { (-weight as u32) - 1 } else { 0 };

        for (i, digit) in dec.into_iter().enumerate() {
            let fract_pow = 4 * (i as u32 + 1 + start_fractionals);

            if fract_pow <= MAX_PRECISION {
                result += Decimal::new(digit as i64, 0) / Decimal::from_i128_with_scale(10i128.pow(fract_pow), 0);
            } else if fract_pow == MAX_PRECISION + 4 {
                // rounding last digit
                if digit >= 5000 {
                    result += Decimal::new(1 as i64, 0) / Decimal::from_i128_with_scale(10i128.pow(MAX_PRECISION), 0);
                }
            }
        }
    }

    result.set_sign_negative(neg);

    // Rescale to the postgres value, automatically rounding as needed.
    result.rescale(scale as u32);

    Ok(result)
}

fn to_postgres(decimal: Decimal) -> PostgresDecimal<Vec<i16>> {
    if decimal.is_zero() {
        return PostgresDecimal {
            neg: false,
            weight: 0,
            scale: 0,
            digits: vec![0],
        };
    }

    let scale = decimal.scale() as u16;

    // A serialized version of the decimal number. The resulting byte array
    // will have the following representation:
    //
    // Bytes 1-4: flags
    // Bytes 5-8: lo portion of m
    // Bytes 9-12: mid portion of m
    // Bytes 13-16: high portion of m
    let mut mantissa = u128::from_le_bytes(decimal.serialize());

    // chop off the flags
    mantissa >>= 32;

    // If our scale is not a multiple of 4, we need to go to the next
    // multiple.
    let groups_diff = scale % 4;
    if groups_diff > 0 {
        let remainder = 4 - groups_diff as u32;
        let power = 10u32.pow(remainder as u32) as u128;

        mantissa = mantissa * power;
    }

    // Array to store max mantissa of Decimal in Postgres decimal format.
    let mut digits = Vec::with_capacity(8);

    // Convert to base-10000.
    while mantissa != 0 {
        digits.push((mantissa % 10_000) as i16);
        mantissa /= 10_000;
    }

    // Change the endianness.
    digits.reverse();

    // Weight is number of digits on the left side of the decimal.
    let digits_after_decimal = (scale + 3) as u16 / 4;
    let weight = digits.len() as i16 - digits_after_decimal as i16 - 1;

    // Remove non-significant zeroes.
    while let Some(&0) = digits.last() {
        digits.pop();
    }

    PostgresDecimal {
        neg: decimal.is_sign_negative(),
        scale,
        weight,
        digits,
    }
}

impl<'a> FromSql<'a> for DecimalWrapper {
    // Decimals are represented as follows:
    // Header:
    //  u16 numGroups
    //  i16 weightFirstGroup (10000^weight)
    //  u16 sign (0x0000 = positive, 0x4000 = negative, 0xC000 = NaN)
    //  i16 dscale. Number of digits (in base 10) to print after decimal separator
    //
    //  Pseudo code :
    //  const Decimals [
    //          0.0000000000000000000000000001,
    //          0.000000000000000000000001,
    //          0.00000000000000000001,
    //          0.0000000000000001,
    //          0.000000000001,
    //          0.00000001,
    //          0.0001,
    //          1,
    //          10000,
    //          100000000,
    //          1000000000000,
    //          10000000000000000,
    //          100000000000000000000,
    //          1000000000000000000000000,
    //          10000000000000000000000000000
    //  ]
    //  overflow = false
    //  result = 0
    //  for i = 0, weight = weightFirstGroup + 7; i < numGroups; i++, weight--
    //    group = read.u16
    //    if weight < 0 or weight > MaxNum
    //       overflow = true
    //    else
    //       result += Decimals[weight] * group
    //  sign == 0x4000 ? -result : result

    // So if we were to take the number: 3950.123456
    //
    //  Stored on Disk:
    //    00 03 00 00 00 00 00 06 0F 6E 04 D2 15 E0
    //
    //  Number of groups: 00 03
    //  Weight of first group: 00 00
    //  Sign: 00 00
    //  DScale: 00 06
    //
    // 0F 6E = 3950
    //   result = result + 3950 * 1;
    // 04 D2 = 1234
    //   result = result + 1234 * 0.0001;
    // 15 E0 = 5600
    //   result = result + 5600 * 0.00000001;
    //

    fn from_sql(_: &Type, raw: &[u8]) -> Result<DecimalWrapper, Box<dyn error::Error + 'static + Sync + Send>> {
        let mut raw = Cursor::new(raw);
        let num_groups = raw.read_u16::<BigEndian>()?;
        let weight = raw.read_i16::<BigEndian>()?; // 10000^weight
                                                   // Sign: 0x0000 = positive, 0x4000 = negative, 0xC000 = NaN
        let sign = raw.read_u16::<BigEndian>()?;

        // Number of digits (in base 10) to print after decimal separator
        let scale = raw.read_u16::<BigEndian>()?;

        // Read all of the groups
        let mut groups = Vec::new();
        for _ in 0..num_groups as usize {
            groups.push(raw.read_u16::<BigEndian>()?);
        }

        let dec = from_postgres(PostgresDecimal {
            neg: sign == 0x4000,
            weight,
            scale,
            digits: groups.into_iter(),
        })
        .map_err(Box::new)?;

        Ok(DecimalWrapper(dec))
    }

    fn accepts(ty: &Type) -> bool {
        match ty {
            &Type::NUMERIC => true,
            _ => false,
        }
    }
}

impl ToSql for DecimalWrapper {
    fn to_sql(&self, _: &Type, out: &mut BytesMut) -> Result<IsNull, Box<dyn error::Error + 'static + Sync + Send>> {
        let PostgresDecimal {
            neg,
            weight,
            scale,
            digits,
        } = to_postgres(self.0);

        let num_digits = digits.len();

        // Reserve bytes
        out.reserve(8 + num_digits * 2);

        // Number of groups
        out.put_u16(num_digits.try_into().unwrap());

        // Weight of first group
        out.put_i16(weight);

        // Sign
        out.put_u16(if neg { 0x4000 } else { 0x0000 });

        // DScale
        out.put_u16(scale);

        // Now process the number
        for digit in digits[0..num_digits].iter() {
            out.put_i16(*digit);
        }

        Ok(IsNull::No)
    }

    fn accepts(ty: &Type) -> bool {
        match ty {
            &Type::NUMERIC => true,
            _ => false,
        }
    }

    to_sql_checked!();
}
