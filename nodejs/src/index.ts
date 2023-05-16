import EventEmitter from 'node:events'
import path from 'node:path'
import os from 'node:os'
import { setImmediate } from 'node:timers/promises'

import { DefaultLibraryLoader } from './engines/DefaultLibraryLoader'
import { LibraryEngine } from './engines/LibraryEngine'
import { disabledTracingHelper } from './engines/TracingHelper'
import { Closeable, Queryable } from './engines/types/Library'
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
  const connectionString = `${process.env.TEST_DATABASE_URL as string}/test`

  /* Use `mock` if you want to test local promises with no database */
  const mock = createMockQueryable(connectionString)

  /* Use `db` if you want to test the actual MySQL database */
  const db = createMySQLQueryable(connectionString)

  // `binder` is required to preserve the `this` context to the group of functions passed to libquery.
  const nodejsFnCtx = binder(db)

  // I assume nobody will run this on Windows ¯\_(ツ)_/¯
  const libExt = os.platform() === 'darwin' ? 'dylib' : 'so'
  const libQueryEnginePath = path.join(__dirname, `../../target/debug/libquery_engine.${libExt}`)

  const schemaPath = path.join(__dirname, `../prisma/schema.prisma`)

  const logEmitter = new EventEmitter().on('error', () => {})

  const engineConfig = {
    nodejsFnCtx,
    cwd: process.cwd(),
    dirname: __dirname,
    enableDebugLogs: true,
    allowTriggerPanic: false,
    datamodelPath: schemaPath,
    prismaPath: libQueryEnginePath,
    showColors: false,
    logLevel: 'info' as const,
    logQueries: false,
    env: {},
    flags: [],
    clientVersion: 'x.y.z',
    previewFeatures: ['node-drivers'],
    activeProvider: 'mysql',
    tracingHelper: disabledTracingHelper,
    logEmitter: logEmitter,
    engineProtocol: 'json' as const,
    isBundled: false,
  }

  const libraryLoader = new DefaultLibraryLoader(engineConfig, libQueryEnginePath)
  const engine = new LibraryEngine(engineConfig, libraryLoader)

  // wait for the database pool to be initialized
  await setImmediate(0)

  // call the `engine.engine?.testAsync` function if you want to quickly test the async/await functionality.
  console.log('calling test_async')
  // @ts-ignore: 
  const result = await engine.engine?.testAsync('SELECT id, firstname, company_id FROM some_user');
  console.log('called test_async', result)

  await nodejsFnCtx.close()
}

main().catch((e) => {
  console.error(e)
  process.exit(1)
})
