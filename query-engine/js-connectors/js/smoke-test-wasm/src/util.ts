import type { Closeable, Connector } from '@jkomyno/prisma-js-connector-utils'
import { QueryEngineInstance } from './engines/types/Library'
import datamodel from '../prisma/postgres-neon/schema.prisma'
import init, * as libqueryEngine from './pkg/libquery_wasm'
import wasm from './pkg/libquery_wasm_bg.wasm?module'

export async function initQueryEngine(driver: Connector & Closeable): Promise<QueryEngineInstance> {
  await init(wasm)
  libqueryEngine.initPanicHook()

  const QueryEngine = libqueryEngine.QueryEngine

  const queryEngineOptions = {
    datamodel,
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

  const driver1 = new libqueryEngine.JsQueryable(new libqueryEngine.Proxy(driver.queryRaw, driver.executeRaw, driver.version, driver.close, driver.isHealthy, "postgres"), "postgres")
  const engine = new QueryEngine(queryEngineOptions, logCallback, driver1)

  return engine
}
