import * as pg from '@jkomyno/prisma-pg-js-connector'
import * as lib from './engines/Library'
import * as os from 'node:os'
import * as path from 'node:path'
import * as fs from 'node:fs'

export function initQueryEngine(driver: pg.Connector, schemaPath: string): lib.QueryEngineInstance {
    // I assume nobody will run this on Windows ¯\_(ツ)_/¯
    const libExt = os.platform() === 'darwin' ? 'dylib' : 'so'
    const dirname = path.dirname(new URL(import.meta.url).pathname)

    const libQueryEnginePath = path.join(dirname, `../../../../../target/debug/libquery_engine.${libExt}`)

    console.log('[nodejs] read Prisma schema from', schemaPath)

    const libqueryEngine = { exports: {} as unknown as lib.Library }
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

    const logCallback = (...args) => {
        console.log(args)
    }
    const engine = new QueryEngine(queryEngineOptions, logCallback, driver)

    return engine
}
