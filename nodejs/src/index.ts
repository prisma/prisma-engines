import path from 'node:path'
import os from 'node:os'

import { DefaultLibraryLoader } from './engines/DefaultLibraryLoader'
import { LibraryEngine } from './engines/LibraryEngine'
import EventEmitter from 'events'
import { disabledTracingHelper } from './engines/TracingHelper'
import { NodejsFunctionContext } from './engines/types/Library'

async function main() {
  const nodejsFnCtx: NodejsFunctionContext = {
    queryRaw: () => Promise.resolve({ data: ['query_raw', 'this is from Node.js'] }),
    queryRawTyped: () => Promise.resolve({ data: ['query_raw_typed', 'this is from Node.js'] }),
    executeRaw: () => Promise.resolve({ data: ['execute_raw', 'this is from Node.js'] }),
    executeRawTyped: () => Promise.resolve({ data: ['execute_raw_typed', 'this is from Node.js'] }),
    version: () => 'x.y.z',
  }

  // I assume nobody will run this on Windows ¯\_(ツ)_/¯
  const libExt = os.platform() === 'darwin' ? 'dylib' : 'so'
  const libQueryEnginePath = path.join(__dirname, `../../target/release/libquery_engine.${libExt}`)

  const schemaPath = path.join(__dirname, `../prisma/schema.prisma`)

  const logEmitter = new EventEmitter().on('error', () => {
    // this is a no-op to prevent unhandled error events
    //
    // If the user enabled error logging this would never be executed. If the user did not
    // enabled error logging, this would be executed, and a trace for the error would be logged
    // in debug mode, which is like going in the opposite direction than what the user wanted by
    // not enabling error logging in the first place.
  })

  const engineConfig = {
    nodejsFnCtx,
    cwd: process.cwd(),
    dirname: __dirname,
    enableDebugLogs: true,
    allowTriggerPanic: false,
    datamodelPath: schemaPath,
    prismaPath: libQueryEnginePath,
    showColors: false,
    logLevel: 'info' as const,
    logQueries: false,
    env: {},
    flags: [],
    clientVersion: 'x.y.z',
    previewFeatures: ['node-drivers'],
    activeProvider: 'mysql',
    tracingHelper: disabledTracingHelper,
    logEmitter: logEmitter,
    engineProtocol: 'json' as const,
    isBundled: false,
  }

  const libraryLoader = new DefaultLibraryLoader(engineConfig, libQueryEnginePath)
  const engine = new LibraryEngine(engineConfig, libraryLoader)

  console.log('engine', engine)
}

main().catch((e) => {
  console.error(e)
  process.exit(1)
})
