/**
 * Run with: `node --experimental-wasm-modules ./example.js`
 * on Node.js 18+.
 */

import { Pool, neonConfig } from '@neondatabase/serverless'
import { PrismaNeon } from '@prisma/adapter-neon'
import { bindAdapter } from '@prisma/driver-adapter-utils'
import { init, QueryEngine, getBuildTimeInfo } from './pkg/query_engine.js'
import { WebSocket  } from 'undici'

neonConfig.webSocketConstructor = WebSocket

async function main() {
  // Always initialize the Wasm library before using it.
  // This sets up the logging and panic hooks.
  init()

  const connectionString = process.env.DATABASE_URL

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
  const res = await queryEngine.query(JSON.stringify({
    modelName: 'User',
    action: 'findMany',
    query: {
      arguments: {},
      selection: {
        $scalars: true
      }
    }
  }), 'trace')
  const parsed = JSON.parse(res);
  console.log('query result = ', parsed)

  const error = parsed.errors?.[0]?.user_facing_error
  if (error?.error_code === 'P2036') {
    console.log('js error:', driverAdapter.errorRegistry.consumeError(error.meta.id))
  }
  // if (res.error.user_facing_error.code =)
  await queryEngine.disconnect('trace')
  console.log('after disconnect')
  queryEngine.free()
  await driverAdapter.close()
}

main()
