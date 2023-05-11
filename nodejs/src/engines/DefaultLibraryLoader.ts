import os from 'os'
import { Library, LibraryLoader } from './types/Library'
import { EngineConfig } from './types/Engine'

export function load(libraryPath: string) {
  const libraryModule = { exports: {} }

  let flags = 0

  if (process.platform !== 'win32') {
    // Add RTLD_LAZY and RTLD_DEEPBIND on Unix.
    //
    // RTLD_LAZY: this is what Node.js uses by default on all Unix-like systems
    // if no flags were passed to dlopen from JavaScript side.
    //
    // RTLD_DEEPBIND: this is not a part of POSIX standard but a widely
    // supported extension. It prevents issues when we dynamically link to
    // system OpenSSL on Linux but the dynamic linker resolves the symbols from
    // the Node.js binary instead.
    //
    // @ts-expect-error TODO: typings don't define dlopen -- needs to be fixed upstream
    flags = os.constants.dlopen.RTLD_LAZY | os.constants.dlopen.RTLD_DEEPBIND
  }

  // @ts-expect-error TODO: typings don't define dlopen -- needs to be fixed upstream
  process.dlopen(libraryModule, libraryPath, flags)
  return libraryModule.exports as Library
}

export class DefaultLibraryLoader implements LibraryLoader {
  private config: EngineConfig
  private libQueryEnginePath: string

  constructor(config: EngineConfig, libQueryEnginePath) {
    this.config = config
    this.libQueryEnginePath = libQueryEnginePath
  }

  async loadLibrary(): Promise<Library> {
    const enginePath = this.libQueryEnginePath
    return this.config.tracingHelper.runInChildSpan('loadLibrary', () => load(enginePath))
  }
}
