//! Note: Only number operations are implemented at the moment.
use super::*;
use std::ops::*;

/// Used right now to reduce code duplication, probably needs to be scrapped once we need anything beyond that.
macro_rules! number_operation {
  ($trait_:ident, $fname:ident, $op:tt) => {
    impl $trait_ for PrismaValue {
      type Output = PrismaValue;

      fn $fname(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
          (PrismaValue::Null, _) | (_, PrismaValue::Null) => PrismaValue::Null,

          (PrismaValue::Int(l), PrismaValue::Int(r)) => PrismaValue::Int(l $op r),
          (PrismaValue::Int(l), PrismaValue::Float(r)) => {
              PrismaValue::Int(l $op r.to_i64().expect("Unable to convert decimal to i64"))
          }

          (PrismaValue::Float(l), PrismaValue::Int(r)) => PrismaValue::Float(
              l $op (Decimal::from_i64(r).expect("Invalid i64 to decimal conversion.")),
          ),

          (PrismaValue::Float(l), PrismaValue::Float(r)) => PrismaValue::Float(l $op r),

          _ => unimplemented!(),
        }
      }
    }
  }
}

number_operation!(Add, add, +);
number_operation!(Sub, sub, -);
number_operation!(Div, div, /);
number_operation!(Mul, mul, *);
