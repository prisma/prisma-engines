import { ErrorRegistryImplementation, wrapAsync } from './errors';
import type { BoundConnector, Connector, Transaction } from './types';

// *.bind(connector) is required to preserve the `this` context of functions whose
// execution is delegated to napi.rs.
export const bindConnector = (connector: Connector): BoundConnector => {
  const errorRegistry = new ErrorRegistryImplementation()

  return {
    errorRegistry,
    queryRaw: wrapAsync(errorRegistry,  connector.queryRaw.bind(connector)),
    executeRaw: wrapAsync(errorRegistry, connector.executeRaw.bind(connector)),
    flavour: connector.flavour,
    startTransaction: async (...args) => {
      const result = await connector.startTransaction(...args);
      if (result.ok) {
        return { ok: true, result: bindTransaction(errorRegistry, result.result)}
      }
      return result
    },
    close: wrapAsync(errorRegistry, connector.close.bind(connector))
  }
}

// *.bind(transaction) is required to preserve the `this` context of functions whose
// execution is delegated to napi.rs.
const bindTransaction = (errorRegistry: ErrorRegistryImplementation, transaction: Transaction): Transaction => {
  return ({
    flavour: transaction.flavour,
    queryRaw: wrapAsync(errorRegistry, transaction.queryRaw.bind(transaction)),
    executeRaw: wrapAsync(errorRegistry, transaction.executeRaw.bind(transaction)),
    commit: wrapAsync(errorRegistry, transaction.commit.bind(transaction)),
    rollback: wrapAsync(errorRegistry, transaction.rollback.bind(transaction))
  });
}