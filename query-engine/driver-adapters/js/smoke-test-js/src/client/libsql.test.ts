import { PrismaLibsql } from '@aqrln/prisma-adapter-libsql'
import { IntMode, createClient } from '@libsql/client'
import { describe } from 'node:test'
import { smokeTestClient } from './client'

describe('libsql with @prisma/client', async () => {
  const connectionString = process.env.JS_LIBSQL_DATABASE_URL as string
  const authToken = process.env.JS_LIBSQL_AUTH_TOKEN
  const intMode = process.env.JS_LIBSQL_INT_MODE as IntMode | undefined

  const client = createClient({ url: connectionString, authToken, intMode })
  const adapter = new PrismaLibsql(client)

  smokeTestClient(adapter)
})
