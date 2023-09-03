import type { ErrorCapturingConnector, Connector, Transaction, ErrorRegistry, ErrorRecord, Result } from './types';


class ErrorRegistryInternal implements ErrorRegistry {
  private registeredErrors: ErrorRecord[] = []

  consumeError(id: number): ErrorRecord | undefined {
      return this.registeredErrors[id]
  }

  registerNewError(error: unknown) {
      let i=0;
      while (this.registeredErrors[i] !== undefined) {
          i++
      }
      this.registeredErrors[i] = { error }
      return i
  }

}

// *.bind(connector) is required to preserve the `this` context of functions whose
// execution is delegated to napi.rs.
export const bindConnector = (connector: Connector): ErrorCapturingConnector => {
  const errorRegistry = new ErrorRegistryInternal()

  return {
    errorRegistry,
    queryRaw: wrapAsync(errorRegistry,  connector.queryRaw.bind(connector)),
    executeRaw: wrapAsync(errorRegistry, connector.executeRaw.bind(connector)),
    flavour: connector.flavour,
    startTransaction: async (...args) => {
      const result = await connector.startTransaction(...args);
      if (result.ok) {
        return { ok: true, value: bindTransaction(errorRegistry, result.value)}
      }
      return result
    },
    close: wrapAsync(errorRegistry, connector.close.bind(connector))
  }
}

// *.bind(transaction) is required to preserve the `this` context of functions whose
// execution is delegated to napi.rs.
const bindTransaction = (errorRegistry: ErrorRegistryInternal, transaction: Transaction): Transaction => {
  return ({
    flavour: transaction.flavour,
    queryRaw: wrapAsync(errorRegistry, transaction.queryRaw.bind(transaction)),
    executeRaw: wrapAsync(errorRegistry, transaction.executeRaw.bind(transaction)),
    commit: wrapAsync(errorRegistry, transaction.commit.bind(transaction)),
    rollback: wrapAsync(errorRegistry, transaction.rollback.bind(transaction))
  });
}

function wrapAsync<A extends unknown[], R>(registry: ErrorRegistryInternal, fn: (...args: A) => Promise<Result<R>>): (...args: A) => Promise<Result<R>> {
  return async (...args) => {
      try {
          return await fn(...args)
      } catch (error) {
          const id = registry.registerNewError(error)
          return { ok: false, error: { kind: 'GenericJsError', id } }
      }
  }
}