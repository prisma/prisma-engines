import {
  type SqlQuery,
  type Result,
  type SqlResultSet,
  type ErrorCapturingDriverAdapter,
  ok,
} from '@prisma/driver-adapter-utils'

type Recordings = ReturnType<typeof createInMemoryRecordings>

export function recording(adapter: ErrorCapturingDriverAdapter) {
  const recordings = createInMemoryRecordings()

  return {
    recorder: recorder(adapter, recordings),
    replayer: replayer(adapter, recordings),
    recordings: recordings,
  }
}

function recorder(
  adapter: ErrorCapturingDriverAdapter,
  recordings: Recordings,
) {
  return {
    provider: adapter.provider,
    adapterName: adapter.adapterName,
    transactionContext: () => {
      throw new Error('Not implemented')
    },
    executeScript: async () => {
      throw new Error('Not implemented')
    },
    getConnectionInfo: () => {
      return adapter.getConnectionInfo!()
    },
    queryRaw: async (params) => {
      const result = await adapter.queryRaw(params)
      recordings.addQueryResults(params, result)
      return result
    },
    executeRaw: async (params) => {
      throw new Error('Not implemented')
    },
    dispose: async () => {
      await adapter.dispose()
      return ok(undefined)
    },
    errorRegistry: adapter.errorRegistry,
  } satisfies ErrorCapturingDriverAdapter
}

function replayer(
  adapter: ErrorCapturingDriverAdapter,
  recordings: Recordings,
) {
  return {
    provider: adapter.provider,
    adapterName: adapter.adapterName,
    recordings: recordings,
    transactionContext: () => {
      throw new Error('Not implemented')
    },
    executeScript: async () => {
      throw new Error('Not implemented')
    },
    getConnectionInfo: () => {
      return adapter.getConnectionInfo!()
    },
    queryRaw: async (params) => {
      return recordings.getQueryResults(params)
    },
    executeRaw: async (params) => {
      return recordings.getCommandResults(params)
    },
    dispose: async () => {
      await adapter.dispose()
      return ok(undefined)
    },
    errorRegistry: adapter.errorRegistry,
  } satisfies ErrorCapturingDriverAdapter & { recordings: Recordings }
}

function createInMemoryRecordings() {
  const queryResults: Map<string, Result<SqlResultSet>> = new Map()
  const commandResults: Map<string, Result<number>> = new Map()

  const queryToKey = (params: SqlQuery) => {
    var sql = params.sql
    params.args.forEach((arg: any, i) => {
      sql = sql.replace('$' + (i + 1), arg.toString())
    })
    return sql
  }

  return {
    data: (): Map<string, SqlResultSet> => {
      const map = new Map()
      for (const [key, value] of queryResults.entries()) {
        value.map((resultSet) => {
          map[key] = resultSet
        })
      }
      return map
    },

    addQueryResults: (params: SqlQuery, result: Result<SqlResultSet>) => {
      const key = queryToKey(params)
      queryResults.set(key, result)
    },

    getQueryResults: (params: SqlQuery) => {
      const key = queryToKey(params)

      if (!queryResults.has(key)) {
        throw new Error(`SqlQuery not recorded: ${key}`)
      }

      return queryResults.get(key)!
    },

    addCommandResults: (params: SqlQuery, result: Result<number>) => {
      const key = queryToKey(params)
      commandResults.set(key, result)
    },

    getCommandResults: (params: SqlQuery) => {
      const key = queryToKey(params)

      if (!commandResults.has(key)) {
        throw new Error(`Command not recorded: ${key}`)
      }

      return commandResults.get(key)!
    },
  }
}
