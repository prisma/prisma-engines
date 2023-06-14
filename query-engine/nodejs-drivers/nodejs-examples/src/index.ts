import path from 'node:path'
import os from 'node:os'
import fs from 'node:fs'
import { setImmediate, setTimeout } from 'node:timers/promises'

import { Closeable, Library, Queryable } from './engines/types/Library'
import { createMySQLQueryable } from './queryable/mysql'
import { createMockQueryable } from './queryable/mock'

// *.bind(db) is required to preserve the `this` context.
// There are surely other ways than this to use class methods defined in JS within a
// napi.rs context, but this is the most straightforward.
const binder = (queryable: Queryable & Closeable): Queryable & Closeable => ({
  queryRaw: queryable.queryRaw.bind(queryable),
  executeRaw: queryable.executeRaw.bind(queryable),
  version: queryable.version.bind(queryable),
  isHealthy: queryable.isHealthy.bind(queryable),
  close: queryable.close.bind(queryable),
})

async function main() {
  const connectionString = `${process.env.TEST_DATABASE_URL as string}`

  /* Use `mock` if you want to test local promises with no database */
  const mock = createMockQueryable(connectionString)

  /* Use `db` if you want to test the actual MySQL database */
  const db = createMySQLQueryable(connectionString)

  // `binder` is required to preserve the `this` context to the group of functions passed to libquery.
  const nodejsFnCtx = binder(db)

  // wait for the database pool to be initialized
  await setImmediate(0)

  // I assume nobody will run this on Windows ¯\_(ツ)_/¯
  const libExt = os.platform() === 'darwin' ? 'dylib' : 'so'
  const libQueryEnginePath = path.join(__dirname, `../../../../target/debug/libquery_engine.${libExt}`)
  
  const schemaPath = path.join(__dirname, `../prisma/schema.prisma`)

  const libqueryEngine = { exports: {} as unknown as Library} 
  // @ts-ignore
  process.dlopen(libqueryEngine, libQueryEnginePath)

  const QueryEngine = libqueryEngine.exports.QueryEngine

  const queryEngineOptions = {
    datamodel: fs.readFileSync(schemaPath, 'utf-8'),
    configDir: '.',
    engineProtocol: 'json' as const,
    logLevel: 'info' as const,
    logQueries: false,
    env: process.env,
    ignoreEnvVarErrors: false,
  }

  const logCallback = (...args) => console.log(args)
  const engine = new QueryEngine(queryEngineOptions, logCallback, nodejsFnCtx)

  console.log(engine)

  console.log('[nodejs] connecting...')
  await engine.connect('trace')
  console.log('[nodejs] connected')

  const resultSet = await engine.query(`{
    "modelName": "some_user",
    "action": "findMany",
    "query": {
        "selection": {
          "id": true,
          "firstname": true,
          "company_id": true
        }
      } 
    }`, 'trace', undefined)

  console.log('[nodejs] resultSet', resultSet)

  // Note: calling `engine.disconnect` won't actually close the database connection.
  console.log('[nodejs] disconnecting...')
  await engine.disconnect('trace')
  console.log('[nodejs] disconnected')

  console.log('[nodejs] connecting...')
  await engine.connect('trace')
  console.log('[nodejs] connecting')

  await setTimeout(2000)

  console.log('[nodejs] disconnecting...')
  await engine.disconnect('trace')
  console.log('[nodejs] disconnected')

  // Close the database connection. This is required to prevent the process from hanging.
  console.log('[nodejs] closing database connection...')
  await nodejsFnCtx.close()
  console.log('[nodejs] closed database connection')

  process.exit(0)
}

main().catch((e) => {
  console.error(e)
  process.exit(1)
})
