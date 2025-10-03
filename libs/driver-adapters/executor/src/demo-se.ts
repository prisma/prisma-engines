import * as S from '@effect/schema/Schema'
import { bindSqlAdapterFactory } from '@prisma/driver-adapter-utils'
import process from 'node:process'

import type { DriverAdaptersManager } from './driver-adapters-manager/index.js'
import { Env } from './types/index.js'
import * as se from './schema-engine-wasm-module.js'
import { setupDriverAdaptersManager } from './setup.js'
import {
  getWasmError,
  isWasmPanic,
  WasmPanicRegistry,
} from './wasm-panic-registry.js'

/**
 * Set up a global registry for Wasm panics.
 * This allows us to retrieve the panic message from the Wasm panic hook,
 * which is not possible otherwise.
 */
globalThis.PRISMA_WASM_PANIC_REGISTRY = new WasmPanicRegistry()

/**
 * Example run: `EXTERNAL_TEST_EXECUTOR="Wasm" DRIVER_ADAPTER="d1" pnpm demo:se`
 */
async function main(): Promise<void> {
  const env = S.decodeUnknownSync(Env)(process.env)
  console.log('[env]', env)

  /**
   * Static input for demo purposes.
   */

  const url = process.env.TEST_DATABASE_URL!

  const schema = /* prisma */ `
    generator client {
      provider = "prisma-client"
    }

    datasource db {
      provider = "sqlite"
      url      = "${url}"
    }

    model User {
      id Int @id @default(autoincrement())
      email String @unique
      name String?
      posts Post[]
    }

    model Post {
      id Int @id @default(autoincrement())
      title String
      content String
      author User @relation(fields: [authorId], references: [id])
      authorId Int
    }
  `

  const driverAdapterManager = await setupDriverAdaptersManager(env, { url })

  const { engine } = await initSE({
    driverAdapterManager,
    options: {
      datamodels: [[schema, 'schema.prisma']],
    },
  })

  {
    console.log('[version]')
    const version = await engine.version()
    console.dir({ version }, { depth: null })
  }

  // {
  //   console.log('[devDiagnostic]')
  //   const result = await engine.devDiagnostic({
  //     migrationsList: {
  //       baseDir: process.cwd(),
  //       lockfile: {
  //         path: 'migrations_lock.toml',
  //         content: null,
  //       },
  //       migrationDirectories: [],
  //     },
  //   })
  //   console.dir({ result }, { depth: null })
  // }

  {
    console.log('[ensureConnectionValidity]')
    const result = await engine.ensureConnectionValidity({
      datasource: {
        tag: 'Schema',
        files: [
          {
            content: schema,
            path: 'schema.prisma',
          },
        ],
      },
    })
    console.dir({ result }, { depth: null })
  }

  {
    console.log('[reset]')
    const result = await engine.reset()
    console.dir({ result }, { depth: null })
  }

  {
    console.log('[db push]')
    const result = await engine.schemaPush({
      schema: {
        files: [
          {
            content: schema,
            path: 'schema.prisma',
          },
        ],
      },
      force: false,
    })
    console.dir({ result }, { depth: null })
  }

  {
    console.log('[reset]')
    const result = await engine.reset()
    console.dir({ result }, { depth: null })
  }

  {
    console.log('[diff from empty to schemaDatamodel]')
    const diffResult = await engine.diff({
      from: {
        tag: 'empty',
      },
      to: {
        tag: 'schemaDatamodel',
        files: [
          {
            content: schema,
            path: 'schema.prisma',
          },
        ],
      },
      exitCode: null,
      shadowDatabaseUrl: null,
      script: true,
    })
    console.dir({ diffResult }, { depth: null })
  }

  {
    console.log('[introspect]')
    const introspectResult = await engine.introspect({
      schema: {
        files: [
          {
            content: schema,
            path: 'schema.prisma',
          },
        ],
      },
      baseDirectoryPath: process.cwd(),
      compositeTypeDepth: 0,
      force: false,
      namespaces: null,
    })
    console.dir(introspectResult, { depth: null })
  }

  {
    console.log('[reset]')
    const result = await engine.reset()
    console.dir({ result }, { depth: null })
  }
}

type InitSchemaEngineParams = {
  driverAdapterManager: DriverAdaptersManager
  options: se.ConstructorOptions
}

async function initSE({
  driverAdapterManager,
  options,
}: InitSchemaEngineParams) {
  const adapterFactory = driverAdapterManager.factory()
  const errorCapturingAdapterFactory = bindSqlAdapterFactory(adapterFactory)

  const debug = (log: string) => {
    console.log('[debug]')
    console.dir(JSON.parse(log), { depth: null })
  }

  const engineInstance = await se.initSchemaEngine(
    options,
    debug,
    errorCapturingAdapterFactory,
  )

  return {
    engine: engineInstance,
    adapterFactory: errorCapturingAdapterFactory,
  }
}

process.on('uncaughtException', (error: Error) => {
  console.log('[uncaughtException]')

  if (isWasmPanic(error)) {
    const { message, stack } = getWasmError(
      globalThis.PRISMA_WASM_PANIC_REGISTRY,
      error,
    )

    console.error('[WasmPanic]', { message, stack })
  } else {
    console.error('[Error]', error)
  }

  process.exit(1)
})

process.on('unhandledRejection', (error: Error) => {
  console.log('[unhandledRejection]')

  if (isWasmPanic(error)) {
    const { message, stack } = getWasmError(
      globalThis.PRISMA_WASM_PANIC_REGISTRY,
      error,
    )

    console.error('[WasmPanic]', { message, stack })
  } else {
    console.error('[Error]', error)
  }

  process.exit(2)
})

main()
