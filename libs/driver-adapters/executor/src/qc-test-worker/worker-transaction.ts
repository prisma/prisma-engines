import { UserFacingError } from '@prisma/client-engine'
import type { IsolationLevel } from '@prisma/driver-adapter-utils'
import type { State } from './worker.js'
import { TxOptions } from '../types/jsonRpc.js'

export function parseIsolationLevel(
  level: string | null | undefined,
): IsolationLevel | undefined {
  if (level == null) {
    return undefined
  }

  switch (level.toLowerCase()) {
    case 'readcommitted':
    case 'read committed':
      return 'READ COMMITTED'

    case 'readuncommitted':
    case 'read uncommitted':
      return 'READ UNCOMMITTED'

    case 'repeatableread':
    case 'repeatable read':
      return 'REPEATABLE READ'

    case 'serializable':
      return 'SERIALIZABLE'

    case 'snapshot':
      return 'SNAPSHOT'

    default:
      // We don't validate the isolation level on the RPC schema level because some tests
      // rely on sending invalid isolation levels to test error handling, and those invalid
      // levels must be forwarded to the query engine as-is in `testd-qe.ts`.
      throw new Error(`Invalid isolation level \`${level}\``)
  }
}

export type TransactionInfo = {
  id: string
}

type UserFacingErrorObject = ReturnType<
  UserFacingError['toQueryResponseErrorObject']
>['user_facing_error']

export async function startTransaction(
  options: TxOptions,
  state: State,
): Promise<TransactionInfo | UserFacingErrorObject> {
  return await handleUserFacingError(() =>
    state.transactionManager.startTransaction({
      maxWait: options.max_wait,
      timeout: options.timeout,
      isolationLevel: parseIsolationLevel(options.isolation_level),
    }),
  )
}

export async function commitTransaction(
  txId: string,
  state: State,
): Promise<Record<PropertyKey, never> | UserFacingErrorObject> {
  return await handleUserFacingError(async () => {
    await state.transactionManager.commitTransaction(txId)
    return {}
  })
}

export async function rollbackTransaction(
  txId: string,
  state: State,
): Promise<Record<PropertyKey, never> | UserFacingErrorObject> {
  return await handleUserFacingError(async () => {
    await state.transactionManager.rollbackTransaction(txId)
    return {}
  })
}

async function handleUserFacingError<T>(
  fn: () => Promise<T>,
): Promise<T | UserFacingErrorObject> {
  try {
    return await fn()
  } catch (error) {
    if (error instanceof UserFacingError) {
      return error.toQueryResponseErrorObject().user_facing_error
    }
    throw error
  }
}
