// Same order as in rust js-connectors' `ColumnType`.
// Note: exporting const enums causes lots of problems with bundlers, so we emulate
// them via regular dictionaries.
// See: https://hackmd.io/@dzearing/Sk3xV0cLs
export const ColumnTypeEnum = {
  'Int32': 0,
  'Int64': 1,
  'Float': 2,
  'Double': 3,
  'Numeric': 4,
  'Boolean': 5,
  'Char': 6,
  'Text': 7,
  'Date': 8,
  'Time': 9,
  'DateTime': 10,
  'Json': 11,
  'Enum': 12,
  'Bytes': 13,
  // 'Set': 14,
  // 'Array': 15,
  // ...
} as const

export const connectionHealthErrorCodes = [
  // Unable to resolve the domain name to an IP address.
  'ENOTFOUND',

  // Failed to get a response from the DNS server.
  'EAI_AGAIN',

  // The connection was refused by the database server.
  'ECONNREFUSED',

  // The connection attempt timed out.
  'ETIMEDOUT',

  // The connection was unexpectedly closed by the database server.
  'ECONNRESET',
] as const
