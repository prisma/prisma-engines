let NEXT_REQUEST_ID = 1n
const MAX_REQUEST_ID = 0xffffffffffffffffn

export function nextRequestId(): string {
  const id = NEXT_REQUEST_ID.toString()
  NEXT_REQUEST_ID++
  if (NEXT_REQUEST_ID > MAX_REQUEST_ID) {
    NEXT_REQUEST_ID = 1n
  }
  return id
}
