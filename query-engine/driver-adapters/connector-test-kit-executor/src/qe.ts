import type { ErrorCapturingDriverAdapter } from '@prisma/driver-adapter-utils'
import { WasmQueryEngine } from './wasm'
import * as napi from './engines/Library'
import * as os from 'node:os'
import * as path from 'node:path'
import { fileURLToPath } from 'node:url'

const dirname = path.dirname(fileURLToPath(import.meta.url))

export interface QueryEngine {
  connect(trace: string): Promise<void>
  disconnect(trace: string): Promise<void>;
  query(body: string, trace: string, tx_id?: string): Promise<string>;
  startTransaction(input: string, trace: string): Promise<string>;
  commitTransaction(tx_id: string, trace: string): Promise<string>;
  rollbackTransaction(tx_id: string, trace: string): Promise<string>;
}

export type QueryLogCallback = (log: string) => void


export function initQueryEngine(adapter: ErrorCapturingDriverAdapter, datamodel: string, queryLogCallback: QueryLogCallback, debug: (...args: any[]) => void): QueryEngine {

    const queryEngineOptions = {
        datamodel,
        configDir: '.',
        engineProtocol: 'json' as const,
        logLevel: process.env["RUST_LOG"] ?? 'info' as any,
        logQueries: true,
        env: process.env,
        ignoreEnvVarErrors: false,
    }


    const logCallback = (event: any) => {
        const parsed = JSON.parse(event)
        if (parsed.is_query) {
            queryLogCallback(parsed.query)
        }
        debug(parsed)
    }

    const engineFromEnv = process.env.EXTERNAL_TEST_EXECUTOR_ENGINE ?? 'napi'
    if (engineFromEnv === 'WASM') {
        return  new WasmQueryEngine(queryEngineOptions, logCallback, adapter)
    } else if (engineFromEnv === 'NAPI') {
        const { QueryEngine } = loadNapiEngine()
        return new QueryEngine(queryEngineOptions, logCallback, adapter)
    } else {
        throw new TypeError(`Invalid EXTERNAL_TEST_EXECUTOR_ENGINE value: ${engineFromEnv}. Expected NAPI or WASM`)
    }


}

function loadNapiEngine(): napi.Library {
    // I assume nobody will run this on Windows ¯\_(ツ)_/¯
    const libExt = os.platform() === 'darwin' ? 'dylib' : 'so'

    const libQueryEnginePath = path.join(dirname, `../../../../target/debug/libquery_engine.${libExt}`)

    const libqueryEngine = { exports: {} as unknown as napi.Library }
    // @ts-ignore
    process.dlopen(libqueryEngine, libQueryEnginePath)

    return libqueryEngine.exports
}