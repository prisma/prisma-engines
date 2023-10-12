import { connect } from '@tidbcloud/serverless'
import { PrismaTiDBCloud } from '@prisma/adapter-tidbcloud'
import { bindAdapter } from '@prisma/driver-adapter-utils'
import { describe } from 'node:test'
import { smokeTestLibquery } from './libquery'

describe('tidbcloud', () => {
  const connectionString = process.env.JS_TIDBCLOUD_DATABASE_URL ?? ''

  const connnection = connect({ url: connectionString })
  const adapter = new PrismaTiDBCloud(connnection)
  const driverAdapter = bindAdapter(adapter)

  smokeTestLibquery(driverAdapter, '../../prisma/mysql/schema.prisma')
})
