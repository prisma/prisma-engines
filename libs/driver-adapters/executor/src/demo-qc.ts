import * as S from '@effect/schema/Schema'
import { bindAdapter, ConnectionInfo } from '@prisma/driver-adapter-utils'

import type { DriverAdaptersManager } from './driver-adapters-manager'
import { Env } from './types'
import * as qc from './query-compiler'
import { err } from './utils'
import { setupDriverAdaptersManager } from './setup'

/**
 * Example run: `DRIVER_ADAPTER="libsql" pnpm demo:qc`
 */
async function main(): Promise<void> {
  const env = S.decodeUnknownSync(Env)(process.env)
  console.log('[env]', env)

  /**
   * Static input for demo purposes.
   */

  const url = 'file:./db.sqlite'

  const schema = /* prisma */ `
    generator client {
      provider = "prisma-client-js"
    }

    datasource db {
      provider = "sqlite"
      url      = "file:./db.sqlite"
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

  const { compiler: compiler } = await initQC({
    driverAdapterManager,
    schema,
  })

  const query = compiler.compile(
    JSON.stringify({
      modelName: 'User',
      action: 'createOne',
      query: {
        arguments: {
          data: {
            email: 'whatever@gmail.com',
          },
        },
        selection: {
          id: true,
        },
      },
    }),
  )
  console.log('[query]', query)
}

type InitQueryCompilerParams = {
  driverAdapterManager: DriverAdaptersManager
  schema: string
}

async function initQC({
  driverAdapterManager,
  schema,
}: InitQueryCompilerParams) {
  const adapter = await driverAdapterManager.connect()
  const errorCapturingAdapter = bindAdapter(adapter)

  let connectionInfo: ConnectionInfo = {}
  if (errorCapturingAdapter.getConnectionInfo) {
    const result = errorCapturingAdapter.getConnectionInfo()
    if (!result.ok) {
      throw result.error
    }
    connectionInfo = result.value
  }

  const compiler = await qc.initQueryCompiler({
    datamodel: schema,
    provider: adapter.provider,
    connectionInfo,
  })

  return {
    compiler: compiler,
    adapter: errorCapturingAdapter,
  }
}

main().catch(err)
