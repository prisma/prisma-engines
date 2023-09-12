import { it } from 'node:test'
import { createClient } from '@libsql/client'
import { PrismaLibsql } from '../dist/index.js'

function connect(): PrismaLibsql {
  const client = createClient({ url: 'file:test.db' })
  return new PrismaLibsql(client)
}

it('raw', async () => {
  const client = createClient({ url: 'file:test.db', intMode: 'string' })

  await client.execute(`
    DROP TABLE IF EXISTS types
 `)

  await client.execute(`
    CREATE TABLE types (
      id     INTEGER PRIMARY KEY,
      int    INTEGER,
      float1 real,
      float2 DOUBLE,
      date   DATETIME,
      text   TEXT,
      blob   BLOB
    )
 `)

  await client.execute({
    sql: `
      INSERT INTO types (
        int,
        blob
      ) VALUES (
        9223372036854775807,
        ?
      )
    `,
    args: [Buffer.from('abcd')]
  })

  const r = await client.execute(`SELECT * FROM types`)
  console.log(r)

  const r2 = await client.execute(`SELECT 1`)
  console.log(r2)

  const r3 = await client.execute(`SELECT *, COUNT(*) FROM types`)
  console.log(r3)
})

it('declared types', async () => {
  const client = connect()

  await client.executeRaw({
    sql: `
      DROP TABLE IF EXISTS types;
    `,
    args: [],
  })

  await client.executeRaw({
    sql: `
      CREATE TABLE types (
        id     INTEGER PRIMARY KEY,
        float1 REAL,
        float2 DOUBLE,
        date   DATETIME,
        text   TEXT,
        blob   BLOB
      )
    `,
    args: [],
  })
})
