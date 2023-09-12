import { ColumnTypeEnum, ColumnType } from '@prisma/driver-adapter-utils'
import { Value } from '@libsql/client'

class UnexpectedTypeError extends Error {
  name = 'UnexpectedTypeError'
  constructor(value: unknown) {
    const type = typeof value
    const repr = type === 'object' ? JSON.stringify(value) : String(value)
    super(`unexpected value of type ${type}: ${repr}`)
  }
}

/**
 * This is currently based on the values since libsql doesn't expose column types.
 */
export function fieldToColumnType(fieldValue: Value): ColumnType {
  if (fieldValue === null) {
    // TODO: not much we can do without decltype
    return ColumnTypeEnum.Int32
  }

  switch (typeof fieldValue) {
    case 'string':
      return ColumnTypeEnum.Text
    case 'bigint':
      return ColumnTypeEnum.Int64
    case 'boolean':
      return ColumnTypeEnum.Boolean
    case 'number':
      return inferNumericType(fieldValue)
    case 'object':
      return inferObjectType(fieldValue)
    default:
      throw new UnexpectedTypeError(fieldValue)
  }
}

function inferNumericType(value: number): ColumnType {
  if (!Number.isInteger(value)) {
    return ColumnTypeEnum.Double
  }
  if (value >= -0x80000000 && value <= 0x7fffffff) {
    return ColumnTypeEnum.Int32
  } else {
    return ColumnTypeEnum.Int64
  }
}

function inferObjectType(value: {}): ColumnType {
  if (value instanceof ArrayBuffer || value[Symbol.toStringTag] === 'ArrayBuffer') {
    return ColumnTypeEnum.Bytes
  }
  throw new UnexpectedTypeError(value)
}
