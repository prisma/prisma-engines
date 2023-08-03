import path from 'node:path'
import fs from 'node:fs'

import { Connector, QueryEngineInstance } from './engines/types/Library.js'
import init, * as libqueryEngine from './pkg/query_engine.js'
import wasm from './pkg/query_engine_bg.wasm?module'
import datamodel from '../prisma/schema.prisma'

export async function initQueryEngine(driver: Connector): QueryEngineInstance {
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

  const driver1 = new libqueryEngine.JsQueryable(new libqueryEngine.Proxy(driver.queryRaw, driver.executeRaw, driver.version, driver.close, driver.isHealthy, driver.flavor), driver.flavor)
  const engine = new QueryEngine(queryEngineOptions, logCallback, driver1)

  return engine
}
