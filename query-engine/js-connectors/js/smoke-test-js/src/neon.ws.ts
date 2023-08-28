import ws from 'ws'
import { Pool, neonConfig } from '@neondatabase/serverless'
import { NeonWSAdapter } from '@jkomyno/prisma-neon-driver-adapter'
import { bindConnector } from '@jkomyno/prisma-js-connector-utils'
import { smokeTest } from './test'

async function neonWS() {
  neonConfig.webSocketConstructor = ws

  const connectionString = `${process.env.JS_NEON_DATABASE_URL as string}`

  const neonPool = new Pool({
    connectionString,
  })

  const adapter = new NeonWSAdapter(neonPool)

  const db = bindConnector(adapter)
  await smokeTest(db, '../prisma/postgres-neon/schema.prisma')
}

neonWS().catch((e) => {
  console.error(e)
  process.exit(1)
})
