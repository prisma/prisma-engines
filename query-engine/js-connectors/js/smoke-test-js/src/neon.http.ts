import { neon } from '@neondatabase/serverless'
import { NeonHTTPAdapter } from '@jkomyno/prisma-neon-driver-adapter'
import { bindConnector } from '@jkomyno/prisma-js-connector-utils'
import { smokeTest } from './test'

async function neonHTTP() {
  const connectionString = `${process.env.JS_NEON_DATABASE_URL as string}`

  const neonConnection = neon(connectionString, {
    arrayMode: false,
    fullResults: true,
  })

  const adapter = new NeonHTTPAdapter(neonConnection)

  const db = bindConnector(adapter)
  await smokeTest(db, '../prisma/postgres-neon/schema.prisma')
}

neonHTTP().catch((e) => {
  console.error(e)
  process.exit(1)
})
