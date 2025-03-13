import { IsolationLevel } from '@prisma/driver-adapter-utils'
import type { State } from './worker'
import { TxOptions } from '../types/jsonRpc'

export function parseIsolationLevel(
  level: string | null | undefined,
): IsolationLevel | undefined {
  if (level == null) {
    return undefined
  }

  switch (level.toLowerCase()) {
    case 'readcommitted':
    case 'read committed':
      return IsolationLevel.ReadCommitted

    case 'readuncommitted':
    case 'read uncommitted':
      return IsolationLevel.ReadUncommitted

    case 'repeatableread':
    case 'repeatable read':
      return IsolationLevel.RepeatableRead

    case 'serializable':
      return IsolationLevel.Serializable

    case 'snapshot':
      return IsolationLevel.Snapshot

    default:
      // We don't validate the isolation level on the RPC schema level because some tests
      // rely on sending invalid isolation levels to test error handling, and those invalid
      // levels must be forwarded to the query engine as-is in `testd-qe.ts`.
      throw new Error(`Unknown isolation level: ${level}`)
  }
}

export type TransactionInfo = {
  id: string
}

export async function startTransaction(
  options: TxOptions,
  state: State,
): Promise<TransactionInfo> {
  return await state.transactionManager.startTransaction({
    maxWait: options.max_wait,
    timeout: options.timeout,
    isolationLevel: parseIsolationLevel(options.isolation_level),
  })
}

export async function commitTransaction(
  txId: string,
  state: State,
): Promise<void> {
  await state.transactionManager.commitTransaction(txId)
}

export async function rollbackTransaction(
  txId: string,
  state: State,
): Promise<void> {
  await state.transactionManager.rollbackTransaction(txId)
}
