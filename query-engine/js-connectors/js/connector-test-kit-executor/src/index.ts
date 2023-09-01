import * as pg from '@jkomyno/prisma-pg-js-connector'
import * as qe from './qe'
import * as engines from './engines/Library'
import * as readline from 'node:readline'
import * as jsonRpc from './jsonRpc'
import * as tempy from 'tempy'

async function main(): Promise<void> {
    const url = process.env["TEST_DATABASE_URL"]

    if (!url) {
        throw new Error("The TEST_DATABASE_URL environment variable is not defined.")
    }

    const iface = readline.createInterface({
        input: process.stdin,
        output: process.stdout,
        terminal: false,
    });

    iface.on('line', async (line) => {
        try {
            const request: jsonRpc.Request = JSON.parse(line); // todo: validate
            try {
                const response = await handleRequest(request.method, request.params)
                respondOk(request.id, response)
            } catch (err) {
                respondErr(request.id, {
                    code: 1,
                    message: err.toString(),
                })
            }
        } catch (_) {
            // skip non-JSON line
        }

    });
}

const schemas: Record<number, engines.QueryEngineInstance> = {}

async function handleRequest(method: string, params: unknown): Promise<unknown> {
    switch (method) {
        case 'initialize': {
            return { datamodel_provider: "postgresql" }
        }
        case 'initializeSchema': {
            interface InitializeSchemaParams {
                schema: string
                schemaId: number
                url: string
            }

            const castParams = params as InitializeSchemaParams;
            const engine = await initQe(castParams.url, castParams.schema);
            await engine.connect("")
            schemas[castParams.schemaId] = engine
            return null
        }
        case 'query': {
            interface QueryPayload {
                query: string
                schemaId: number
            }

            const castParams = params as QueryPayload;
            const result = await schemas[castParams.schemaId].query(castParams.query, "")

            return JSON.parse(result)
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

async function initQe(url: string, prismaSchema: string): Promise<engines.QueryEngineInstance> {
    const connector = pg.createPgConnector({
        url,
    });
    const schemaPath: string = await tempy.temporaryWrite(prismaSchema);
    return qe.initQueryEngine(connector, schemaPath)
}

main().catch(console.error)
