import { ColumnTypeEnum, type ColumnType } from '@jkomyno/prisma-js-connector-utils'

function isDateTimeString(input: string) {
  // Regular expression to match the format "YYYY-MM-DD HH:MM:SS"
  const dateRegex = /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/;

  if (!dateRegex.test(input)) {
    return false; // Doesn't match the expected format
  }

  const date = new Date(input);
  return !isNaN(date.getTime()) && dateRegex.test(input);
}

/**
 * This is a simplification of quaint's value inference logic. Take a look at quaint's conversion.rs
 * module to see how other attributes of the field packet such as the field length are used to infer
 * the correct quaint::Value variant.
 */
export function resultToColumnType(key, value): ColumnType {
  if(!value)
    return ColumnTypeEnum.Text

  switch (typeof value) {
    case "string":
      // special handling for DateTimes
      if (isDateTimeString(value)) {
        return ColumnTypeEnum.DateTime
      }
      // console.log(`${value} => Text`)
      return ColumnTypeEnum.Text
    case "number":
      // console.log(`${value} => Int32`)
      return ColumnTypeEnum.Int32

      /*
      return ColumnTypeEnum.Int32
      return ColumnTypeEnum.Int64
      return ColumnTypeEnum.Float
      return ColumnTypeEnum.Double
      return ColumnTypeEnum.Boolean
      return ColumnTypeEnum.Date
      return ColumnTypeEnum.Time
      return ColumnTypeEnum.DateTime
      return ColumnTypeEnum.Numeric
      return ColumnTypeEnum.Char
      return ColumnTypeEnum.Text
      */

    default:
      console.log("conversion.ts: !!! Unsupported column type", typeof value, value)
      throw new Error(`Unsupported column type: ${typeof value}`)
  }
}