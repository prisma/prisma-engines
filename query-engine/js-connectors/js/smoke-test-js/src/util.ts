import path from 'node:path'
import os from 'node:os'
import fs from 'node:fs'
import type { Connector } from '@jkomyno/prisma-js-connector-utils'
import { Library, QueryEngineInstance } from './engines/types/Library.js'
import { QueryEngine, Proxy as ConnectorProxy, initPanicHook } from './wasm/libquery_wasm.js'

const dirname = path.dirname(new URL(import.meta.url).pathname)

const defaultQueryEngineOptions = {
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

export function initQueryEngine(driver: Connector, prismaSchemaRelativePath: string): QueryEngineInstance {
  const profile = 'debug'
  
  // I assume nobody will run this on Windows ¯\_(ツ)_/¯
  const libExt = os.platform() === 'darwin' ? 'dylib' : 'so'

  const basePath = '../../../../..'
  const libQueryEnginePath = path.join(dirname, `${basePath}/target/${profile}/libquery_engine.${libExt}`)
  const schemaPath = path.join(dirname, prismaSchemaRelativePath)
  console.log('[nodejs] read Prisma schema from', schemaPath)

  const libqueryEngine = { exports: {} as unknown as Library }
  // @ts-ignore
  process.dlopen(libqueryEngine, libQueryEnginePath)

  const QueryEngine = libqueryEngine.exports.QueryEngine

  const queryEngineOptions = {
    ...defaultQueryEngineOptions,
    datamodel: fs.readFileSync(schemaPath, 'utf-8'),
  }

  const engine = new QueryEngine(queryEngineOptions, logCallback, driver)

  return engine
}

export function initQueryEngineWasm(driver: Connector, prismaSchemaRelativePath: string): QueryEngine {
  const schemaPath = path.join(dirname, prismaSchemaRelativePath)
  console.log('[nodejs] read Prisma schema from', schemaPath)

  const queryEngineOptions = {
    datamodel: fs.readFileSync(schemaPath, 'utf-8'),
    configDir: '.',
    engineProtocol: 'json' as const,
    logLevel: 'info' as const,
    logQueries: false,
    env: process.env,
    ignoreEnvVarErrors: false,
  }

  // TODO: capture Wasm panics similarly to how we do in prisma/prisma for `@prisma/prisma-schema-wasm`.
  // Should we combine that with an EventEmitter?
  initPanicHook()

  const proxy = new ConnectorProxy(driver)
  const engine = new QueryEngine(queryEngineOptions, logCallback, proxy)

  return engine
}
