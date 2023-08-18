import type { Connector, Transaction } from './types';

// *.bind(db) is required to preserve the `this` context.
// There are surely other ways than this to use class methods defined in JS within a
// driver context, but this is the most straightforward.
export const bindConnector = (connector: Connector): Connector => ({
  queryRaw: connector.queryRaw.bind(connector),
  executeRaw: connector.executeRaw.bind(connector),
  flavour: connector.flavour,
  startTransaction: connector.startTransaction.bind(connector),
  close: connector.close.bind(connector)
})

// *.bind(transaction) is required to preserve the `this` context of functions whose
// execution is delegated to napi.rs.
export const bindTransaction = (transaction: Transaction): Transaction => {
  return ({
    flavour: transaction.flavour,
    queryRaw: transaction.queryRaw.bind(transaction),
    executeRaw: transaction.executeRaw.bind(transaction),
    commit: transaction.commit.bind(transaction),
    rollback: transaction.rollback.bind(transaction)
  });
}