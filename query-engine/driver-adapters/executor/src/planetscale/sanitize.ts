// This is a temporary workaround for `bigint` serialization issues in the Planetscale driver,
// which will be fixed upstream once https://github.com/planetscale/database-js/pull/159 is published.
// This only impacts Rust tests concerning `driver-adapters`.

type Stringable = { toString: () => string }
type Value = null | undefined | number | boolean | string | Array<Value> | Date | Stringable

export function format(query: string, values: Value[] | Record<string, Value>): string {
  return Array.isArray(values) ? replacePosition(query, values) : replaceNamed(query, values)
}

function replacePosition(query: string, values: Value[]): string {
  let index = 0
  return query.replace(/\?/g, (match) => {
    return index < values.length ? sanitize(values[index++]) : match
  })
}

function replaceNamed(query: string, values: Record<string, Value>): string {
  return query.replace(/:(\w+)/g, (match, name) => {
    return hasOwn(values, name) ? sanitize(values[name]) : match
  })
}

function hasOwn(obj: unknown, name: string): boolean {
  return Object.prototype.hasOwnProperty.call(obj, name)
}

function sanitize(value: Value): string {
  if (value == null) {
    return 'null'
  }

  if (['number', 'bigint'].includes(typeof value)) {
    return String(value)
  }

  if (typeof value === 'boolean') {
    return value ? 'true' : 'false'
  }

  if (typeof value === 'string') {
    return quote(value)
  }

  if (Array.isArray(value)) {
    return value.map(sanitize).join(', ')
  }

  if (value instanceof Date) {
    return quote(value.toISOString().slice(0, -1))
  }

  return quote(value.toString())
}

function quote(text: string): string {
  return `'${escape(text)}'`
}

const re = /[\0\b\n\r\t\x1a\\"']/g

function escape(text: string): string {
  return text.replace(re, replacement)
}

function replacement(text: string): string {
  switch (text) {
    case '"':
      return '\\"'
    case "'":
      return "\\'"
    case '\n':
      return '\\n'
    case '\r':
      return '\\r'
    case '\t':
      return '\\t'
    case '\\':
      return '\\\\'
    case '\0':
      return '\\0'
    case '\b':
      return '\\b'
    case '\x1a':
      return '\\Z'
    default:
      return ''
  }
}
