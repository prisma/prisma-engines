import { connect } from '@tidbcloud/serverless'
import { PrismaTiDBCloud } from '@prisma/adapter-tidbcloud'
import { describe } from 'node:test'
import { smokeTestClient } from './client'

describe('tidbcloud with @prisma/client', async () => {
  const connectionString = process.env.JS_TIDBCLOUD_DATABASE_URL ?? ''

  const connnection = connect({ url: connectionString })
  const adapter = new PrismaTiDBCloud(connnection)

  smokeTestClient(adapter)
})
