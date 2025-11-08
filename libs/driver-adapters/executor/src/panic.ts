type GlobalWithPanicHandler = typeof globalThis & {
  PRISMA_WASM_PANIC_REGISTRY: {
    set_message?: (message: string) => void
  }
}

const global = globalThis as GlobalWithPanicHandler

export function setupDefaultPanicHandler() {
  global.PRISMA_WASM_PANIC_REGISTRY = {
    set_message(message: string) {
      throw new PanicError(message)
    },
  }
}

export function withLocalPanicHandler<T>(fn: () => T): T {
  const previousHandler = global.PRISMA_WASM_PANIC_REGISTRY.set_message
  let panic: string | undefined = undefined

  global.PRISMA_WASM_PANIC_REGISTRY.set_message = (message) => {
    panic = message
  }

  try {
    return fn()
  } finally {
    global.PRISMA_WASM_PANIC_REGISTRY.set_message = previousHandler

    if (panic) {
      throw new PanicError(panic)
    }
  }
}

export class PanicError extends Error {
  constructor(message: string) {
    super('Panic in Wasm module: ' + message)
    this.name = 'PanicError'
  }
}
