import path from 'node:path'
import fs from 'node:fs'

import { Connector, QueryEngineInstance } from './engines/types/Library.js'
import * as libqueryEngine from './pkg/query_engine.js'

export function initQueryEngine(driver: Connector): QueryEngineInstance {
  libqueryEngine.initPanicHook()
  const dirname = path.dirname(new URL(import.meta.url).pathname)

  const schemaPath = path.join(dirname, `../prisma/schema.prisma`)

  const QueryEngine = libqueryEngine.QueryEngine

  const queryEngineOptions = {
    datamodel: fs.readFileSync(schemaPath, 'utf-8'),
    configDir: '.',
    engineProtocol: 'json' as const,
    logLevel: 'info' as const,
    logQueries: false,
    env: process.env,
    ignoreEnvVarErrors: false,
  }

  const logCallback = (...args) => {
    console.log(args)
  }

  const driver1 = new libqueryEngine.JsQueryable(new libqueryEngine.Proxy(driver.queryRaw, driver.executeRaw, driver.version, driver.close, driver.isHealthy, driver.flavor), driver.flavor)
  const engine = new QueryEngine(queryEngineOptions, logCallback, driver1)

  return engine
}
