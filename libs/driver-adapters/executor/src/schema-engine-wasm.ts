<<<<<<< HEAD:libs/driver-adapters/executor/src/schema-engine-wasm.ts
import fs from 'node:fs/promises'
import path from 'node:path'
import { __dirname, normaliseProvider } from './utils.js'
import type { Queryable } from '@prisma/driver-adapter-utils'

const relativePath = '../../../../schema-engine/schema-engine-wasm/pkg'

const initializedModules = new Set<Queryable['provider']>()

export async function getSchemaEngineForProvider(provider: Queryable['provider']) {
  const normalisedProvider = normaliseProvider(provider)
  const engine = await import(`${relativePath}/${normalisedProvider}/schema_engine_bg.js`)

    if (!initializedModules.has(provider)) {
        const bytes = await fs.readFile(
            path.resolve(
                __dirname,
                relativePath,
                normalisedProvider,
                'schema_engine_bg.wasm',
            ),
        )

=======
import * as wasmPostgres from '../../../../schema-engine/schema-engine-wasm/pkg/postgresql/schema_engine_bg.js'
import * as wasmMysql from '../../../../schema-engine/schema-engine-wasm/pkg/mysql/schema_engine_bg.js'
import * as wasmSqlite from '../../../../schema-engine/schema-engine-wasm/pkg/sqlite/schema_engine_bg.js'
import fs from 'node:fs/promises'
import path from 'node:path'
import { __dirname } from './utils.js'

const wasm = {
    postgres: wasmPostgres,
    mysql: wasmMysql,
    sqlite: wasmSqlite
}

type EngineName = keyof typeof wasm

const initializedModules = new Set<EngineName>()

export async function getSchemaEngineForProvider(provider: EngineName) {
    const engine = wasm[provider]
    if (!initializedModules.has(provider)) {
        const subDir = provider === 'postgres' ? 'postgresql' : provider
        const bytes = await fs.readFile(path.resolve(__dirname, '..', '..', '..', '..', 'schema-engine', 'schema-engine-wasm', 'pkg', subDir, 'schema_engine_bg.wasm'))
>>>>>>> main:query-engine/driver-adapters/executor/src/schema-engine-wasm.ts
        const module = new WebAssembly.Module(bytes)
        const instance = new WebAssembly.Instance(module, { './schema_engine_bg.js': engine })
        engine.__wbg_set_wasm(instance.exports)
        initializedModules.add(provider)
    }

    return engine.SchemaEngine
}
