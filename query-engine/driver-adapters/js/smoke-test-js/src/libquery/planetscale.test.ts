import { connect } from '@planetscale/database'
import { PrismaPlanetScale } from '@prisma/adapter-planetscale'
import { bindAdapter } from '@prisma/driver-adapter-utils'
import { describe } from 'node:test'
import { smokeTestLibquery } from './libquery'

describe('planetscale', () => {
  const connectionString = process.env.JS_PLANETSCALE_DATABASE_URL ?? ''

  const connnection = connect({ url: connectionString })
  const adapter = new PrismaPlanetScale(connnection)
  const driverAdapter = bindAdapter(adapter)

  smokeTestLibquery(driverAdapter, '../../prisma/mysql/schema.prisma')
})
