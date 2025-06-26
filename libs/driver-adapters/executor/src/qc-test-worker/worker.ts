import { Schema as S } from '@effect/schema'
import type {
  ConnectionInfo,
  SqlDriverAdapter,
} from '@prisma/driver-adapter-utils'
import { DriverAdaptersManager } from '../driver-adapters-manager/index.js'
import * as qc from '../query-compiler.js'
import {
  noopTracingHelper,
  TransactionManager,
  type TransactionOptions,
} from '@prisma/client-engine-runtime'
import { parentPort } from 'worker_threads'
import {
  CommitTxParams,
  InitializeSchemaParams,
  QueryParams,
  RollbackTxParams,
  StartTxParams,
} from '../types/jsonRpc.js'
import { assertNever, debug } from '../utils.js'
import { setupDriverAdaptersManager } from '../setup.js'
import { Env } from '../types/index.js'
import { query } from './worker-query.js'
import {
  commitTransaction,
  rollbackTransaction,
  startTransaction,
} from './worker-transaction.js'
import { setupDefaultPanicHandler } from '../panic.js'

const InitializeSchemaMessage = S.struct({
  type: S.literal('initializeSchema'),
  responsePort: S.instanceOf(MessagePort),
  params: InitializeSchemaParams,
  env: Env,
})

const QueryMessage = S.struct({
  type: S.literal('query'),
  responsePort: S.instanceOf(MessagePort),
  params: QueryParams,
})

const StartTransactionMessage = S.struct({
  type: S.literal('startTx'),
  responsePort: S.instanceOf(MessagePort),
  params: StartTxParams,
})

const CommitTransactionMessage = S.struct({
  type: S.literal('commitTx'),
  responsePort: S.instanceOf(MessagePort),
  params: CommitTxParams,
})

const RollbackTransactionMessage = S.struct({
  type: S.literal('rollbackTx'),
  responsePort: S.instanceOf(MessagePort),
  params: RollbackTxParams,
})

const TeardownMessage = S.struct({
  type: S.literal('teardown'),
  responsePort: S.instanceOf(MessagePort),
})

const GetLogsMessage = S.struct({
  type: S.literal('getLogs'),
  responsePort: S.instanceOf(MessagePort),
})

const Message = S.union(
  InitializeSchemaMessage,
  QueryMessage,
  StartTransactionMessage,
  CommitTransactionMessage,
  RollbackTransactionMessage,
  TeardownMessage,
  GetLogsMessage,
)

export type Message = S.Schema.Type<typeof Message>

export type State = {
  compiler: qc.QueryCompiler
  driverAdapterManager: DriverAdaptersManager
  driverAdapter: SqlDriverAdapter
  transactionManager: TransactionManager
}

let state: State | undefined
const logs: string[] = []

if (!parentPort) {
  throw new Error('This module must be run in a worker thread')
}

setupDefaultPanicHandler()

parentPort.on('message', async (rawMsg: unknown) => {
  const msg = S.decodeUnknownSync(Message)(rawMsg)
  let response: unknown

  debug('worker received message:', msg.type)

  try {
    response = await dispatchMessage(msg)
  } catch (error) {
    // TODO: we should have a nicer mapping for driver adapter errors
    response = error instanceof Error ? error : new Error(JSON.stringify(error))
  }

  // The Rust side expects `TransactionEndResponse::Ok(Empty)`,
  // where `Empty` is `struct Empty {}` as an empty response.
  // Without this conversion the test cases don't receive the
  // response at all, rendering them frozen.
  if (response === undefined) {
    response = {}
  }

  debug('worker response:', JSON.stringify(response))

  msg.responsePort.postMessage(response)
})

async function dispatchMessage(msg: Message): Promise<unknown> {
  switch (msg.type) {
    case 'initializeSchema':
      return initializeSchema(msg.params, msg.env)
    case 'query':
      return query(msg.params, unwrapState(), logs)
    case 'startTx':
      return startTransaction(msg.params.options, unwrapState())
    case 'commitTx':
      return commitTransaction(msg.params.txId, unwrapState())
    case 'rollbackTx':
      return rollbackTransaction(msg.params.txId, unwrapState())
    case 'teardown':
      return teardown(unwrapState())
    case 'getLogs': {
      const clonedLogs = [...logs]
      logs.length = 0
      return clonedLogs
    }
    default:
      assertNever(
        msg,
        `Unknown message type: \`${(msg as { type: unknown }).type}\``,
      )
  }
}

function unwrapState(): State {
  if (state === undefined) {
    throw new Error('State is not initialized, call `initializeSchema` first')
  }
  return state
}

async function initializeSchema(
  params: InitializeSchemaParams,
  env: Env,
): Promise<ConnectionInfo> {
  const { url, schema, migrationScript } = params

  const driverAdapterManager = await setupDriverAdaptersManager(env, {
    url,
    migrationScript,
  })

  const { compiler, adapter } = await initQueryCompiler({
    url,
    driverAdapterManager,
    schema,
  })

  const transactionManager = new TransactionManager({
    driverAdapter: adapter,
    // Transaction timeouts matching those used by the Prisma Client
    transactionOptions: {
      maxWait: 2000,
      timeout: 5000,
    } satisfies TransactionOptions,
    tracingHelper: noopTracingHelper,
  })

  state = {
    compiler,
    driverAdapterManager,
    driverAdapter: adapter,
    transactionManager,
  }

  logs.length = 0

  if (adapter.getConnectionInfo) {
    return adapter.getConnectionInfo()
  }

  return { supportsRelationJoins: false }
}

type InitQueryCompilerParams = {
  driverAdapterManager: DriverAdaptersManager
  url: string
  schema: string
}

async function initQueryCompiler({
  driverAdapterManager,
  schema,
}: InitQueryCompilerParams) {
  const adapter = await driverAdapterManager.connect()

  let connectionInfo: ConnectionInfo = { supportsRelationJoins: false }
  if (adapter.getConnectionInfo) {
    connectionInfo = adapter.getConnectionInfo()
  }

  const compiler = await qc.initQueryCompiler(
    {
      datamodel: schema,
      provider: adapter.provider,
      connectionInfo,
    },
    driverAdapterManager.connector(),
  )

  return {
    compiler,
    adapter,
  }
}

async function teardown(unwrappedState: State) {
  const { compiler, transactionManager, driverAdapterManager } = unwrappedState

  process.nextTick(() => {
    try {
      compiler.free()
    } catch (error) {
      debug('Error dropping compiler:', error)
    }
  })

  await transactionManager.cancelAllTransactions()
  await driverAdapterManager.teardown()

  state = undefined

  return {}
}
