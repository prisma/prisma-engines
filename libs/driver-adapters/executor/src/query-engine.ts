import type { ErrorCapturingSqlDriverAdapter } from '@prisma/driver-adapter-utils'
import * as napi from './engines/Library.js'
import * as os from 'node:os'
import * as path from 'node:path'
import { __dirname } from './utils.js'

export interface QueryEngine {
  connect(trace: string, requestId: string): Promise<void>
  disconnect(trace: string, requestId: string): Promise<void>
  query(
    body: string,
    trace: string,
    tx_id: string | undefined,
    requestId: string,
  ): Promise<string>
  startTransaction(
    input: string,
    trace: string,
    requestId: string,
  ): Promise<string>
  commitTransaction(
    tx_id: string,
    trace: string,
    requestId: string,
  ): Promise<string>
  rollbackTransaction(
    tx_id: string,
    trace: string,
    requestId: string,
  ): Promise<string>
}

export type QueryLogCallback = (log: string) => void

export async function initQueryEngine(
  engineType: 'Napi' | 'Wasm',
  adapter: ErrorCapturingSqlDriverAdapter,
  datamodel: string,
  queryLogCallback: QueryLogCallback,
  debug: (...args: any[]) => void,
): Promise<QueryEngine> {
  const logCallback = (event: any) => {
    const parsed = JSON.parse(event)
    if (parsed.is_query) {
      queryLogCallback(parsed.query)
    }
    debug(parsed)
  }

  const options = queryEngineOptions(datamodel)

  if (engineType === 'Wasm') {
    const { getQueryEngineForProvider: getEngineForProvider } = await import(
      './query-engine-wasm.js'
    )
    const WasmQueryEngine = await getEngineForProvider(adapter.provider)
    return new WasmQueryEngine(options, logCallback, adapter)
  } else {
    const { QueryEngine } = loadNapiEngine()
    return new QueryEngine(options, logCallback, adapter)
  }
}

export function queryEngineOptions(datamodel: string) {
  return {
    datamodel,
    configDir: '.',
    engineProtocol: 'json' as const,
    logLevel: process.env['RUST_LOG'] ?? ('info' as any),
    logQueries: true,
    env: process.env,
    ignoreEnvVarErrors: false,
    enableTracing: true,
  }
}

function loadNapiEngine(): napi.Library {
  // I assume nobody will run this on Windows ¯\_(ツ)_/¯
  const libExt = os.platform() === 'darwin' ? 'dylib' : 'so'
  const target =
    process.env.TARGET || process.env.PROFILE == 'release' ? 'release' : 'debug'

  const libQueryEnginePath = path.resolve(
    __dirname,
    `../../../../target/${target}/libquery_engine.${libExt}`,
  )

  const libqueryEngine = { exports: {} as unknown as napi.Library }
  // @ts-ignore
  process.dlopen(libqueryEngine, libQueryEnginePath)

  return libqueryEngine.exports
}
