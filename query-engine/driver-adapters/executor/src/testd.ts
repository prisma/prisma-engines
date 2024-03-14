import * as readline from 'node:readline'
import { fetch } from 'undici'
import { WebSocket } from 'ws'
import * as S from '@effect/schema/Schema'
import * as prismaPg from '@prisma/adapter-pg'
import * as prismaNeon from '@prisma/adapter-neon'
import { PrismaLibSQL } from '@prisma/adapter-libsql'
import { PrismaPlanetScale } from '@prisma/adapter-planetscale'
import {bindAdapter, DriverAdapter, ErrorCapturingDriverAdapter} from '@prisma/driver-adapter-utils'
import { pg, neon, planetScale, libSql } from '@prisma/bundled-js-drivers'
import { webcrypto } from 'node:crypto'

import { jsonRpc, DriverAdapterTag, Env } from './types'
import * as qe from './qe'

if (!global.crypto) {
  global.crypto = webcrypto as Crypto
}

const SUPPORTED_ADAPTERS: Record<DriverAdapterTag, (_ : string) => Promise<DriverAdapter>>
    = {
        "pg": pgAdapter,
        "neon:ws" : neonWsAdapter,
        "libsql": libsqlAdapter,
        "planetscale": planetscaleAdapter,
    };

// conditional debug logging based on LOG_LEVEL env var
const debug = (() => {
    if ((process.env.LOG_LEVEL ?? '').toLowerCase() != 'debug') {
        return (...args: any[]) => {}
    }

    return (...args: any[]) => {
        console.error('[nodejs] DEBUG:', ...args);
    };
})();

// error logger
const err = (...args: any[]) => console.error('[nodejs] ERROR:', ...args);

async function main(): Promise<void> {
    const env = S.decodeUnknownSync(Env)(process.env)
    console.log('[env]', env)

    const iface = readline.createInterface({
        input: process.stdin,
        output: process.stdout,
        terminal: false,
    });

    iface.on('line', async (line) => {
        try {
            const request = S.decodeSync(jsonRpc.RequestFromString)(line)
            debug(`Got a request: ${line}`)

            try {
                const response = await handleRequest(request, env)
                respondOk(request.id, response)
            } catch (err) {
                debug("[nodejs] Error from request handler: ", err)
                respondErr(request.id, {
                    code: 1,
                    message: err.stack ?? err.toString(),
                })
            }
        } catch (err) {
            console.error("Received non-json line: ", line);
            console.error(err)
        }

    });
}

const state: Record<number, {
    engine: qe.QueryEngine,
    adapter: ErrorCapturingDriverAdapter,
    logs: string[]
}> = {}

async function handleRequest({ method, params }: jsonRpc.Request, env: Env): Promise<unknown> {
    switch (method) {
        case 'initializeSchema': {
            const { url, schema, schemaId } = params
            const logs = [] as string[]

            const logCallback = (log) => { logs.push(log) }

            const { engine, adapter } = await initQe({ env, url, schema, logCallback })
            await engine.connect('')

            state[schemaId] = {
                engine,
                adapter,
                logs
            }
            return null
        }
        case 'query': {
            debug("Got `query`", params)
            const { query, schemaId, txId } = params
            const engine = state[schemaId].engine
            const result = await engine.query(JSON.stringify(query), "", txId)

            const parsedResult = JSON.parse(result)
            if (parsedResult.errors) {
                const error = parsedResult.errors[0]?.user_facing_error
                if (error.error_code === 'P2036') {
                    const jsError = state[schemaId].adapter.errorRegistry.consumeError(error.meta.id)
                    if (!jsError) {
                        err(`Something went wrong. Engine reported external error with id ${error.meta.id}, but it was not registered.`)
                    } else {
                        err("got error response from the engine caused by the driver: ", jsError)
                    }
                }
            }

            debug("got response from engine: ", result)
            // returning unparsed string: otherwise, some information gots lost during this round-trip. 
            // In particular, floating point without decimal part turn into integers
            return result
        }

        case 'startTx': {
            debug("Got `startTx", params)
            const { schemaId, options } = params
            const result = await state[schemaId].engine.startTransaction(JSON.stringify(options), "")
            return JSON.parse(result)
        }

        case 'commitTx': {
            debug("Got `commitTx", params)
            const { schemaId, txId } = params
            const result = await state[schemaId].engine.commitTransaction(txId, '{}')
            return JSON.parse(result)
        }

        case 'rollbackTx': {
            debug("Got `rollbackTx", params)
            const { schemaId, txId } = params
            const result = await state[schemaId].engine.rollbackTransaction(txId, '{}')
            return JSON.parse(result)
        }
        case 'teardown': {
            debug("Got `teardown", params)
            const { schemaId } = params
            await state[schemaId].engine.disconnect("")
            delete state[schemaId]

            return {}
        }
        case 'getLogs': {
            const { schemaId } = params
            return state[schemaId].logs
        }
        default: {
            throw new Error(`Unknown method: \`${method}\``)
        }
    }
}

function respondErr(requestId: number, error: jsonRpc.RpcError) {
    const msg: jsonRpc.ErrResponse = {
        jsonrpc: '2.0',
        id: requestId,
        error,
    }
    console.log(JSON.stringify(msg))
}

function respondOk(requestId: number, payload: unknown) {
    const msg: jsonRpc.OkResponse = {
        jsonrpc: '2.0',
        id: requestId,
        result: payload

    };
    console.log(JSON.stringify(msg))
}

type InitQueryEngineParams = {
    env: Env,
    url: string,
    schema: string,
    logCallback: qe.QueryLogCallback
}

async function initQe({
    env,
    url,
    schema,
    logCallback
}: InitQueryEngineParams) {
    const engineType = env.EXTERNAL_TEST_EXECUTOR ?? 'Napi'
    const adapter = await adapterFromEnv(url) as DriverAdapter
    const errorCapturingAdapter = bindAdapter(adapter)
    const engineInstance = await qe.initQueryEngine(engineType, errorCapturingAdapter, schema, logCallback, debug)
    
    return {
        engine: engineInstance,
        adapter: errorCapturingAdapter,
    }
}

async function adapterFromEnv(url: string): Promise<DriverAdapter> {
    const adapter = process.env.DRIVER_ADAPTER ?? ''

    if (adapter == '') {
        throw new Error("DRIVER_ADAPTER is not defined or empty.")
    }

    if (!(adapter in SUPPORTED_ADAPTERS)) {
        throw new Error(`Unsupported driver adapter: ${adapter}`)
    }

    return await SUPPORTED_ADAPTERS[adapter](url)
}

function postgres_options(url: string): any {
    let args: any = {connectionString: url}
    const schemaName = postgresSchemaName(url)
    if (schemaName != null) {
        args.options = `--search_path="${schemaName}"`
    }
    return args;
}

function postgresSchemaName(url: string) {
    return new URL(url).searchParams.get('schema') ?? undefined
}

async function pgAdapter(url: string): Promise<DriverAdapter> {
    const schemaName = postgresSchemaName(url)
    const pool = new pg.Pool(postgres_options(url))
    return new prismaPg.PrismaPg(pool, {
        schema: schemaName
    })
}

async function neonWsAdapter(url: string): Promise<DriverAdapter> {
    const { neonConfig, Pool: NeonPool } = neon
    const proxyURL = JSON.parse(process.env.DRIVER_ADAPTER_CONFIG || '{}').proxy_url ?? ''
    if (proxyURL == '') {
        throw new Error("DRIVER_ADAPTER_CONFIG is not defined or empty, but its required for neon adapter.");
    }

    neonConfig.wsProxy = () => proxyURL
    neonConfig.webSocketConstructor = WebSocket
    neonConfig.useSecureWebSocket = false
    neonConfig.pipelineConnect = false

    const schemaName = postgresSchemaName(url)

    const pool = new NeonPool(postgres_options(url))
    return new prismaNeon.PrismaNeon(pool, { schema: schemaName })
}

async function libsqlAdapter(url: string): Promise<DriverAdapter> {
    const libsql = libSql.createClient({ url, intMode: 'bigint' })
    return new PrismaLibSQL(libsql)
}

async function planetscaleAdapter(url: string): Promise<DriverAdapter> {
    const proxyUrl = JSON.parse(process.env.DRIVER_ADAPTER_CONFIG || '{}').proxy_url ?? ''
    if (proxyUrl == '') {
        throw new Error("DRIVER_ADAPTER_CONFIG is not defined or empty, but its required for planetscale adapter.");
    }

    const client = new planetScale.Client({
        // preserving path name so proxy url would look like real DB url
        url: copyPathName(url, proxyUrl),
        fetch,
    })

    return new PrismaPlanetScale(client)
}

function copyPathName(fromUrl: string, toUrl: string) {
    const toObj = new URL(toUrl)
    toObj.pathname = new URL(fromUrl).pathname

    return toObj.toString()
}

main().catch(err)
