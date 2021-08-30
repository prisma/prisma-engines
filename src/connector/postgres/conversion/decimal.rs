use bigdecimal::{
    num_bigint::{BigInt, Sign},
    BigDecimal, ToPrimitive, Zero,
};
use byteorder::{BigEndian, ReadBytesExt};
use bytes::{BufMut, BytesMut};
use postgres_types::{to_sql_checked, FromSql, IsNull, ToSql, Type};
use std::{cmp, convert::TryInto, error, fmt, io::Cursor};

#[derive(Debug, Clone)]
pub struct DecimalWrapper(pub BigDecimal);

#[derive(Debug, Clone)]
pub struct InvalidDecimal(&'static str);

impl fmt::Display for InvalidDecimal {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_fmt(format_args!("Invalid Decimal: {}", self.0))
    }
}

impl error::Error for InvalidDecimal {}

struct PostgresDecimal<D> {
    neg: bool,
    weight: i16,
    scale: u16,
    digits: D,
}

fn from_postgres<D: ExactSizeIterator<Item = u16>>(dec: PostgresDecimal<D>) -> Result<BigDecimal, InvalidDecimal> {
    let PostgresDecimal {
        neg, digits, weight, ..
    } = dec;

    if digits.len() == 0 {
        return Ok(0u64.into());
    }

    let sign = match neg {
        false => Sign::Plus,
        true => Sign::Minus,
    };

    // weight is 0 if the decimal point falls after the first base-10000 digit
    let scale = (digits.len() as i64 - weight as i64 - 1) * 4;

    // no optimized algorithm for base-10 so use base-100 for faster processing
    let mut cents = Vec::with_capacity(digits.len() * 2);

    for digit in digits {
        cents.push((digit / 100) as u8);
        cents.push((digit % 100) as u8);
    }

    let bigint = BigInt::from_radix_be(sign, &cents, 100)
        .ok_or(InvalidDecimal("PostgresDecimal contained an out-of-range digit"))?;

    Ok(BigDecimal::new(bigint, scale))
}

fn to_postgres(decimal: &BigDecimal) -> crate::Result<PostgresDecimal<Vec<i16>>> {
    if decimal.is_zero() {
        return Ok(PostgresDecimal {
            neg: false,
            weight: 0,
            scale: 0,
            digits: vec![],
        });
    }

    // NOTE: this unfortunately copies the BigInt internally
    let (integer, exp) = decimal.as_bigint_and_exponent();

    // scale is only nonzero when we have fractional digits
    // since `exp` is the _negative_ decimal exponent, it tells us
    // exactly what our scale should be
    let scale: u16 = cmp::max(0, exp).try_into()?;

    let (sign, uint) = integer.into_parts();
    let mut mantissa = uint.to_u128().unwrap();

    // If our scale is not a multiple of 4, we need to go to the next
    // multiple.
    let groups_diff = scale % 4;
    if groups_diff > 0 {
        let remainder = 4 - groups_diff as u32;
        let power = 10u32.pow(remainder as u32) as u128;

        mantissa *= power;
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

    let neg = match sign {
        Sign::Plus | Sign::NoSign => false,
        Sign::Minus => true,
    };

    Ok(PostgresDecimal {
        neg,
        weight,
        scale,
        digits,
    })
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
        matches!(*ty, Type::NUMERIC)
    }
}

impl ToSql for DecimalWrapper {
    fn to_sql(&self, _: &Type, out: &mut BytesMut) -> Result<IsNull, Box<dyn error::Error + 'static + Sync + Send>> {
        let PostgresDecimal {
            neg,
            weight,
            scale,
            digits,
        } = to_postgres(&self.0)?;

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
        matches!(*ty, Type::NUMERIC)
    }

    to_sql_checked!();
}
