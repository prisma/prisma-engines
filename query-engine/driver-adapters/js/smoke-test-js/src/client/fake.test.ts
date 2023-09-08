import { describe } from 'node:test'
import { PrismaFake } from '@jkomyno/prisma-adapter-fake'
import { smokeTestClient } from './client'

describe('fake with @prisma/client', async () => {
  // const connectionString = `${process.env.JS_PG_DATABASE_URL as string}`

  // const pool = new pg.Pool({ connectionString })
  const adapter = new PrismaFake(undefined)
  
  smokeTestClient(adapter)
})
