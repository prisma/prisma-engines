import { describe } from 'node:test'
import pg from 'pg'
import { PrismaPostgres } from '@jkomyno/prisma-adapter-pg'
import { smokeTestClient } from './client.js'

describe('pg with @prisma/client', async () => {
  const connectionString = `${process.env.JS_PG_DATABASE_URL as string}`

  const pool = new pg.Pool({ connectionString })
  const adapter = new PrismaPostgres(pool)
  
  smokeTestClient(adapter)
})
