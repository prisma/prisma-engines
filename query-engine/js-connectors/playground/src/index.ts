import path from 'node:path'
import os from 'node:os'
import fs from 'node:fs'
import { setImmediate, setTimeout } from 'node:timers/promises'

import type { Closeable, Library, Driver } from './engines/types/Library.js'
import { createPlanetScaleDriver } from './driver/planetscale.js'
import { createMockDriver } from './driver/mock.js'

// *.bind(db) is required to preserve the `this` context.
// There are surely other ways than this to use class methods defined in JS within a
// napi.rs context, but this is the most straightforward.
const binder = (queryable: Driver & Closeable): Driver & Closeable => ({
  queryRaw: queryable.queryRaw.bind(queryable),
  executeRaw: queryable.executeRaw.bind(queryable),
  version: queryable.version.bind(queryable),
  isHealthy: queryable.isHealthy.bind(queryable),
  close: queryable.close.bind(queryable),
})

async function main() {
  const connectionString = `${process.env.TEST_DATABASE_URL as string}`

  /* Use `mock` if you want to test local promises with no database */
  const mock = createMockDriver(connectionString)

  /* Use `db` if you want to test the actual PlanetScale database */
  const db = createPlanetScaleDriver({
    url: connectionString,
  })

  // `binder` is required to preserve the `this` context to the group of functions passed to libquery.
  const nodejsFnCtx = binder(db)

  // wait for the database pool to be initialized
  await setImmediate(0)

  // I assume nobody will run this on Windows ¯\_(ツ)_/¯
  const libExt = os.platform() === 'darwin' ? 'dylib' : 'so'
  const dirname = path.dirname(new URL(import.meta.url).pathname)

  const libQueryEnginePath = path.join(dirname, `../../../../target/debug/libquery_engine.${libExt}`)
  const schemaPath = path.join(dirname, `../prisma/schema.prisma`)

  const libqueryEngine = { exports: {} as unknown as Library }
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
    "modelName": "type_test",
    "action": "findMany",
    "query": {
      "selection": {
        "tinyint_column": true,
        "smallint_column": true,
        "mediumint_column": true,
        "int_column": true,
        "bigint_column": true,
        "float_column": true,
        "double_column": true,
        "decimal_column": true,
        "boolean_column": true,
        "char_column": true,
        "varchar_column": true,
        "text_column": true,
        "date_column": true,
        "time_column": true,
        "datetime_column": true,
        "timestamp_column": true,
        "json_column": true,
        "enum_column": true,
        "binary_column": true,
        "varbinary_column": true,
        "blob_column": true,
        "set_column": true
      }
    } 
    }`, 'trace', undefined)

  console.log('[nodejs] findMany resultSet', JSON.stringify(JSON.parse(resultSet), null, 2))

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
