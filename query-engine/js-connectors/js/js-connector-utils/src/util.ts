import { connectionHealthErrorCodes } from './const'

type ConnectionHealthErrorCode = typeof connectionHealthErrorCodes[number]

export function isConnectionUnhealthy<E extends string>(errorCode: E | ConnectionHealthErrorCode): errorCode is ConnectionHealthErrorCode {
  // Note: `Array.includes` is too narrow, see https://github.com/microsoft/TypeScript/issues/26255.
  return (connectionHealthErrorCodes as readonly string[]).includes(errorCode)
}
