import { PrismaLibsql } from '@aqrln/prisma-adapter-libsql'
import { bindAdapter } from '@jkomyno/prisma-driver-adapter-utils'
import { createClient } from '@libsql/client'
import { describe } from 'node:test'
import { smokeTestLibquery } from './libquery'

describe('libsql', () => {
  const connectionString = process.env.JS_LIBSQL_DATABASE_URL as string
  const authToken = process.env.JS_LIBSQL_AUTH_TOKEN

  const client = createClient({ url: connectionString, authToken: authToken })
  const adapter = new PrismaLibsql(client)
  const driverAdapter = bindAdapter(adapter)

  smokeTestLibquery(driverAdapter, '../../prisma/sqlite/schema.prisma')
})
