import assert from 'node:assert/strict'
import { it } from 'node:test'
import { createClient } from '@libsql/client'
import { PrismaLibsql } from '../dist/index.js'
import { ColumnTypeEnum } from '@jkomyno/prisma-driver-adapter-utils'

function connect(): PrismaLibsql {
  const client = createClient({ url: 'file:test.db' })
  return new PrismaLibsql(client)
}

it('checks declared types', async () => {
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
        real   REAL,
        bigint BIGINT,
        date   DATETIME,
        text   TEXT,
        blob   BLOB
      )
    `,
    args: [],
  })

  const result = await client.queryRaw({
    sql: `
      SELECT * FROM types
    `,
    args: [],
  })

  assert(result.ok)
  assert.deepEqual(result.value.columnTypes, [
    ColumnTypeEnum.Int32,
    ColumnTypeEnum.Double,
    ColumnTypeEnum.Int64,
    ColumnTypeEnum.DateTime,
    ColumnTypeEnum.Text,
    ColumnTypeEnum.Bytes,
  ])
})

it('infers types when sqlite decltype is not available', async () => {
  const client = connect()

  const result = await client.queryRaw({
    sql: `
      SELECT 1 as first, 'test' as second
    `,
    args: [],
  })

  assert(result.ok)
  assert.deepEqual(result.value.columnTypes, [ColumnTypeEnum.Int64, ColumnTypeEnum.Text])
})
