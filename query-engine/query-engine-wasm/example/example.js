/**
 * Run with: `node --experimental-wasm-modules ./example.js`
 * on Node.js 18+.
 */
import { readFile } from 'fs/promises'
import { PrismaLibSQL } from '@prisma/adapter-libsql'
import { createClient } from '@libsql/client'
import { bindAdapter } from '@prisma/driver-adapter-utils'
import { init, QueryEngine, getBuildTimeInfo } from '../pkg/query_engine.js'


async function main() {
  // Always initialize the Wasm library before using it.
  // This sets up the logging and panic hooks.
  init()


  const client = createClient({ url: "file:./prisma/dev.db"})
  const adapter = new PrismaLibSQL(client)
  const driverAdapter = bindAdapter(adapter)

  console.log('buildTimeInfo', getBuildTimeInfo())

  const datamodel = await readFile('prisma/schema.prisma', 'utf8')

  const options = {
    datamodel,
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

  const created = await queryEngine.query(JSON.stringify({
    modelName: 'User',
    action: 'createOne',
    query: {
      arguments: {
        data: {
          id: 1235,
        },
      },
      selection: {
        $scalars: true
      }
    }
  }), 'trace')

  console.log({ created })

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
  console.log('query result = ')
  console.dir(parsed, { depth: null })

  const error = parsed.errors?.[0]?.user_facing_error
  if (error?.error_code === 'P2036') {
    console.log('js error:', driverAdapter.errorRegistry.consumeError(error.meta.id))
  }

  // console.log('before disconnect')
  await queryEngine.disconnect('trace')
  // console.log('after disconnect')

  // console.log('before close')
  await driverAdapter.close()
  // console.log('after close')

  // console.log('before free')
  queryEngine.free()
  // console.log('after free')
}

main()
