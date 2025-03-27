export class WasmPanicRegistry {
  private message = ''

  get() {
    return `${this.message}`
  }

  // Don't use this method directly, it's only used by the Wasm panic hook in @prisma/prisma-schema-wasm.
  private set_message(value: string) {
    this.message = `RuntimeError: ${value}`
  }
}

/**
 * Branded type for Wasm panics.
 */
export type WasmPanic = Error & { name: 'RuntimeError' }

/**
 * Returns true if the given error is a Wasm panic.
 */
export function isWasmPanic(error: Error): error is WasmPanic {
  return error.name === 'RuntimeError'
}

/**
 * Extracts the error message and stack trace from a Wasm panic.
 */
export function getWasmError(wasmPanicRegistry: WasmPanicRegistry, error: WasmPanic) {
  const message: string = wasmPanicRegistry.get()
  const stack = [message, ...(error.stack || 'NO_BACKTRACE').split('\n').slice(1)].join('\n')

  return { message, stack }
}
