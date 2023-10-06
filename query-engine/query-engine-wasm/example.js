/**
 * Run with: `node --experimental-wasm-modules ./example.js`
 * on Node.js 18+.
 */

import { Pool } from '@neondatabase/serverless'
import { PrismaNeon } from '@prisma/adapter-neon'
import { bindAdapter } from '@prisma/driver-adapter-utils'
import { init, QueryEngine, getBuildTimeInfo } from './pkg/query_engine.js'

async function main() {
  // Always initialize the Wasm library before using it.
  // This sets up the logging and panic hooks.
  init()

  const connectionString = undefined

  const pool = new Pool({ connectionString })
  const adapter = new PrismaNeon(pool)
  const driverAdapter = bindAdapter(adapter)

  console.log('buildTimeInfo', getBuildTimeInfo())

  const options = {
    datamodel: /* prisma */`
      datasource db {
        provider = "postgres"
        url      = env("DATABASE_URL")
      }

      generator client {
        provider = "prisma-client-js"
      }

      model User {
        id    Int    @id @default(autoincrement())
      }
    `,
    logLevel: 'info',
    logQueries: true,
    datasourceOverrides: {},
    env: process.env,
    configDir: '/tmp',
    ignoreEnvVarErrors: true,
  }
  const callback = () => { console.log('log-callback') }

  const queryEngine = new QueryEngine(options, callback, driverAdapter)
  
  await queryEngine.connect('trace')
  await queryEngine.disconnect('trace')
}

main()
