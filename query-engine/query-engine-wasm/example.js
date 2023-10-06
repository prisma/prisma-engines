/**
 * Run with: `node --experimental-wasm-modules ./example.js`
 * on Node.js 18+.
 */

import { init, QueryEngine, version } from './pkg/query_engine.js'

async function main() {
  // Always initialize the Wasm library before using it.
  // This sets up the logging and panic hooks.
  init()

  console.log('version', version())

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
  const driverAdapter = undefined

  const queryEngine = new QueryEngine(options, callback, driverAdapter)
  
  queryEngine.connect('trace')
  queryEngine.disconnect('trace')
}

main()
