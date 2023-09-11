import { PrismaLibsql } from '@aqrln/prisma-adapter-libsql'
import { createClient } from '@libsql/client'
import { describe } from 'node:test'
import { smokeTestClient } from './client'

describe('libsql with @prisma/client', async () => {
  const connectionString = process.env.JS_LIBSQL_DATABASE_URL as string
  const authToken = process.env.JS_LIBSQL_AUTH_TOKEN

  const client = createClient({ url: connectionString, authToken })
  const adapter = new PrismaLibsql(client)

  smokeTestClient(adapter)
})
