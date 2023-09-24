import pg from 'pg'
import { PrismaPg } from '@prisma/adapter-pg'
import { bindAdapter } from '@prisma/driver-adapter-utils'
import { describe } from 'node:test'
import { smokeTestLibquery } from './libquery'

// This is currently used to flip between read and write mode for the recordings.
// I could not figure out how to set this via a CLI param with `node --test`
globalThis.recordings = "write"
// globalThis.recordings = "read"

describe('pg', () => {
  const connectionString = process.env.JS_PG_DATABASE_URL ?? ''

  const pool = new pg.Pool({ connectionString })
  const adapter = new PrismaPg(pool)
  const driverAdapter = bindAdapter(adapter)

  smokeTestLibquery(driverAdapter, '../../prisma/postgres/schema.prisma')
})
