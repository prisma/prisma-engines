import fs from 'fs'
import type * as Tx from './types/Transaction'
import { type BeforeExitListener, ExitHooks } from './ExitHooks'
import type { Library, LibraryLoader, QueryEngineConstructor, QueryEngineNodeDriversConstructor, QueryEngineInstance } from './types/Library'
import { BatchQueryEngineResult, DatasourceOverwrite, EngineConfig, EngineEventType, EventEmitter, RequestBatchOptions, RequestOptions } from './types/Engine'
import { EngineBatchQueries, EngineProtocol, EngineQuery, QueryEngineEvent, QueryEngineLogLevel, QueryEnginePanicEvent, QueryEngineQueryEvent, RustRequestError, SyncRustError } from './types/QueryEngine'
import { getBatchRequestPayload } from './utils/getBatchRequestPayload'
import { getInteractiveTransactionId } from './utils/getInteractiveTransactionId'

const debug = console.info

function isQueryEvent(event: QueryEngineEvent): event is QueryEngineQueryEvent {
  return event['item_type'] === 'query' && 'query' in event
}
function isPanicEvent(event: QueryEngineEvent): event is QueryEnginePanicEvent {
  if ('level' in event) {
    return event.level === 'error' && event['message'] === 'PANIC'
  } else {
    return false
  }
}

let engineInstanceCount = 0
const exitHooks = new ExitHooks()

export class LibraryEngine {
  private engine?: QueryEngineInstance
  private libraryInstantiationPromise?: Promise<void>
  private libraryStartingPromise?: Promise<void>
  private libraryStoppingPromise?: Promise<void>
  private libraryStarted: boolean
  private executingQueryPromise?: Promise<any>
  private config: EngineConfig
  private QueryEngineConstructor?: QueryEngineConstructor
  private QueryEngineNodeDriversConstructor?: QueryEngineNodeDriversConstructor
  private libraryLoader: LibraryLoader
  private library?: Library
  private logEmitter: EventEmitter
  private engineProtocol: EngineProtocol
  libQueryEnginePath?: string
  datasourceOverrides: Record<string, string>
  datamodel: string
  logQueries: boolean
  logLevel: QueryEngineLogLevel
  lastQuery?: string
  loggerRustPanic?: any

  versionInfo?: {
    commit: string
    version: string
  }

  get beforeExitListener() {
    return exitHooks.getListener(this)
  }

  set beforeExitListener(listener: BeforeExitListener | undefined) {
    exitHooks.setListener(this, listener)
  }

  constructor(config: EngineConfig, loader: LibraryLoader) {
    try {
      // we try to handle the case where the datamodel is not found
      this.datamodel = fs.readFileSync(config.datamodelPath, 'utf-8')
    } catch (e) {
      if ((e.stack as string).match(/\/\.next|\/next@|\/next\//)) {
        throw new Error(
          `Your schema.prisma could not be found, and we detected that you are using Next.js.
Find out why and learn how to fix this: https://pris.ly/d/schema-not-found-nextjs`,
        )
      } else if (config.isBundled === true) {
        throw new Error(
          `Prisma Client could not find its \`schema.prisma\`. This is likely caused by a bundling step, which leads to \`schema.prisma\` not being copied near the resulting bundle. We would appreciate if you could take the time to share some information with us.
Please help us by answering a few questions: https://pris.ly/bundler-investigation`,
        )
      }

      throw e
    }

    this.config = config
    this.libraryStarted = false
    this.logQueries = config.logQueries ?? false
    this.logLevel = config.logLevel ?? 'error'
    this.libraryLoader = loader
    this.logEmitter = config.logEmitter
    this.engineProtocol = config.engineProtocol
    this.datasourceOverrides = config.datasources ? this.convertDatasources(config.datasources) : {}
    if (config.enableDebugLogs) {
      this.logLevel = 'debug'
    }
    this.libraryInstantiationPromise = this.instantiateLibrary()

    exitHooks.install()
    this.checkForTooManyEngines()
  }

  private checkForTooManyEngines() {
    if (engineInstanceCount === 10) {
      console.warn(`${('warn(prisma-client)')} There are already 10 instances of Prisma Client actively running.`)
    }
  }

  async transaction(
    action: 'start',
    headers: Tx.TransactionHeaders,
    options?: Tx.Options,
  ): Promise<Tx.InteractiveTransactionInfo<undefined>>
  async transaction(
    action: 'commit',
    headers: Tx.TransactionHeaders,
    info: Tx.InteractiveTransactionInfo<undefined>,
  ): Promise<undefined>
  async transaction(
    action: 'rollback',
    headers: Tx.TransactionHeaders,
    info: Tx.InteractiveTransactionInfo<undefined>,
  ): Promise<undefined>
  async transaction(action: any, headers: Tx.TransactionHeaders, arg?: any) {
    await this.start()

    const headerStr = JSON.stringify(headers)

    let result: string | undefined
    if (action === 'start') {
      const jsonOptions = JSON.stringify({
        max_wait: arg?.maxWait ?? 2000, // default
        timeout: arg?.timeout ?? 5000, // default
        isolation_level: arg?.isolationLevel,
      })

      result = await this.engine?.startTransaction(jsonOptions, headerStr)
    } else if (action === 'commit') {
      result = await this.engine?.commitTransaction(arg.id, headerStr)
    } else if (action === 'rollback') {
      result = await this.engine?.rollbackTransaction(arg.id, headerStr)
    }

    const response = this.parseEngineResponse<{ [K: string]: unknown }>(result)

    if (response.error_code) {
      throw new Error(response.message + '\n' + {
        code: response.error_code as string,
        clientVersion: this.config.clientVersion as string,
        meta: response.meta as Record<string, unknown>,
      }.toString())
    }

    return response as Tx.InteractiveTransactionInfo<undefined> | undefined
  }

  private async instantiateLibrary(): Promise<void> {
    debug('internalSetup')
    if (this.libraryInstantiationPromise) {
      return this.libraryInstantiationPromise
    }
    await this.loadEngine()
    this.version()
  }

  private parseEngineResponse<T>(response?: string): T {
    if (!response) {
      throw new Error(`Response from the Engine was empty`)
    }
    try {
      const config = JSON.parse(response)
      return config as T
    } catch (err) {
      throw new Error(`Unable to JSON.parse response from engine`)
    }
  }

  private convertDatasources(datasources: DatasourceOverwrite[]): Record<string, string> {
    const obj = Object.create(null)
    for (const { name, url } of datasources) {
      obj[name] = url
    }
    return obj
  }

  private async loadEngine(): Promise<void> {
    if (!this.engine) {
      this.library = await this.libraryLoader.loadLibrary()
      console.log('Loading Engine...', this.library)

      if (!this.QueryEngineConstructor) {
        this.QueryEngineConstructor = this.library.QueryEngine
      }

      if (!this.QueryEngineNodeDriversConstructor) {
        this.QueryEngineNodeDriversConstructor = this.library.QueryEngineNodeDrivers
      }

      try {
        // Using strong reference to `this` inside of log callback will prevent
        // this instance from being GCed while native engine is alive. At the same time,
        // `this.engine` field will prevent native instance from being GCed. Using weak ref helps
        // to avoid this cycle
        const weakThis = new WeakRef(this)
        this.engine = new this.QueryEngineNodeDriversConstructor(
          {
            datamodel: this.datamodel,
            env: process.env,
            logQueries: this.config.logQueries ?? false,
            ignoreEnvVarErrors: true,
            datasourceOverrides: this.datasourceOverrides,
            logLevel: this.logLevel,
            configDir: this.config.cwd,
            engineProtocol: this.engineProtocol,
          },
          (log) => {
            weakThis.deref()?.logger(log)
          },
          this.config.nodejsFnCtx,
        )
        engineInstanceCount++
      } catch (_e) {
        const e = _e as Error
        const error = this.parseInitError(e.message)
        if (typeof error === 'string') {
          throw e
        } else {
          throw new Error(error.message)
        }
      }
    }
  }

  private logger(log: string) {
    const event = this.parseEngineResponse<QueryEngineEvent | null>(log)
    if (!event) return

    if ('span' in event) {
      this.config.tracingHelper.createEngineSpan()

      return
    }

    event.level = event?.level.toLowerCase() ?? 'unknown'
    if (isQueryEvent(event)) {
      this.logEmitter.emit('query', {
        timestamp: new Date(),
        query: event.query,
        params: event.params,
        duration: Number(event.duration_ms),
        target: event.module_path,
      })
    } else if (isPanicEvent(event)) {
      // The error built is saved to be thrown later
      this.loggerRustPanic = new Error(
        this.getErrorMessageWithLink(
          `${event.message}: ${event.reason} in ${event.file}:${event.line}:${event.column}`,
        ),
      )
    } else {
      this.logEmitter.emit(event.level, {
        timestamp: new Date(),
        message: event.message,
        target: event.module_path,
      })
    }
  }

  private getErrorMessageWithLink(title: string) {
    return {
      title,
      version: this.config.clientVersion!,
      engineVersion: this.versionInfo?.commit,
      database: this.config.activeProvider as any,
      query: this.lastQuery!,
    }.toString()
  }

  private parseInitError(str: string): SyncRustError | string {
    try {
      const error = JSON.parse(str)
      return error
    } catch (e) {
      //
    }
    return str
  }

  private parseRequestError(str: string): RustRequestError | string {
    try {
      const error = JSON.parse(str)
      return error
    } catch (e) {
      //
    }
    return str
  }

  on(event: EngineEventType, listener: (args?: any) => any): void {
    if (event === 'beforeExit') {
      this.beforeExitListener = listener
    } else {
      this.logEmitter.on(event, listener)
    }
  }

  async start(): Promise<void> {
    await this.libraryInstantiationPromise
    await this.libraryStoppingPromise

    if (this.libraryStartingPromise) {
      debug(`library already starting, this.libraryStarted: ${this.libraryStarted}`)
      return this.libraryStartingPromise
    }

    if (this.libraryStarted) {
      return
    }

    const startFn = async () => {
      debug('library starting')

      try {
        const headers = {
          traceparent: this.config.tracingHelper.getTraceParent(),
        }

        await this.engine?.connect(JSON.stringify(headers))

        this.libraryStarted = true

        debug('library started')
      } catch (err) {
        const error = this.parseInitError(err.message as string)

        // The error message thrown by the query engine should be a stringified JSON
        // if parsing fails then we just reject the error
        if (typeof error === 'string') {
          throw err
        } else {
          throw new Error(error.message + '\n' + error.error_code)
        }
      } finally {
        this.libraryStartingPromise = undefined
      }
    }

    this.libraryStartingPromise = this.config.tracingHelper.runInChildSpan('connect', startFn)

    return this.libraryStartingPromise
  }

  async stop(): Promise<void> {
    await this.libraryStartingPromise
    await this.executingQueryPromise

    if (this.libraryStoppingPromise) {
      debug('library is already stopping')
      return this.libraryStoppingPromise
    }

    if (!this.libraryStarted) {
      return
    }

    const stopFn = async () => {
      await new Promise((r) => setTimeout(r, 5))

      debug('library stopping')

      const headers = {
        traceparent: this.config.tracingHelper.getTraceParent(),
      }

      await this.engine?.disconnect(JSON.stringify(headers))

      this.libraryStarted = false
      this.libraryStoppingPromise = undefined

      debug('library stopped')
    }

    this.libraryStoppingPromise = this.config.tracingHelper.runInChildSpan('disconnect', stopFn)

    return this.libraryStoppingPromise
  }

  async getDmmf(): Promise<unknown> {
    await this.start()

    const traceparent = this.config.tracingHelper.getTraceParent()
    const response = await this.engine!.dmmf(JSON.stringify({ traceparent }))

    return this.config.tracingHelper.runInChildSpan('dmmf', () => JSON.parse(response))
  }

  version(): string {
    this.versionInfo = this.library?.version()
    return this.versionInfo?.version ?? 'unknown'
  }

  async request<T>(
    query: EngineQuery,
    { traceparent, interactiveTransaction }: RequestOptions<undefined>,
  ): Promise<{ data: T; elapsed: number }> {
    debug(`sending request, this.libraryStarted: ${this.libraryStarted}`)
    const headerStr = JSON.stringify({ traceparent }) // object equivalent to http headers for the library
    const queryStr = JSON.stringify(query)

    try {
      await this.start()
      this.executingQueryPromise = this.engine?.query(queryStr, headerStr, interactiveTransaction?.id)

      this.lastQuery = queryStr
      const data = this.parseEngineResponse<any>(await this.executingQueryPromise)

      if (data.errors) {
        if (data.errors.length === 1) {
          throw this.buildQueryError(data.errors[0])
        }
        // this case should not happen, as the query engine only returns one error
        throw new Error(JSON.stringify(data.errors))
      } else if (this.loggerRustPanic) {
        throw this.loggerRustPanic
      }
      // TODO Implement Elapsed: https://github.com/prisma/prisma/issues/7726
      return { data, elapsed: 0 }
    } catch (e: any) {
      if (e.code === 'GenericFailure' && e.message?.startsWith('PANIC:')) {
        throw new Error(this.getErrorMessageWithLink(e.message))
      }
      const error = this.parseRequestError(e.message)
      if (typeof error === 'string') {
        throw e
      } else {
        throw new Error(`${error.message}\n${error.backtrace}`)
      }
    }
  }

  async requestBatch<T>(
    queries: EngineBatchQueries,
    { transaction, traceparent }: RequestBatchOptions<undefined>,
  ): Promise<BatchQueryEngineResult<T>[]> {
    debug('requestBatch')
    const request = getBatchRequestPayload(queries, transaction)
    await this.start()

    this.lastQuery = JSON.stringify(request)
    this.executingQueryPromise = this.engine!.query(
      this.lastQuery,
      JSON.stringify({ traceparent }),
      getInteractiveTransactionId(transaction),
    )
    const result = await this.executingQueryPromise
    const data = this.parseEngineResponse<any>(result)

    if (data.errors) {
      if (data.errors.length === 1) {
        throw this.buildQueryError(data.errors[0])
      }
      // this case should not happen, as the query engine only returns one error
      throw new Error(JSON.stringify(data.errors))
    }

    const { batchResult, errors } = data
    if (Array.isArray(batchResult)) {
      return batchResult.map((result) => {
        if (result.errors && result.errors.length > 0) {
          return this.loggerRustPanic ?? this.buildQueryError(result.errors[0])
        }
        return {
          data: result,
          elapsed: 0, // TODO Implement Elapsed: https://github.com/prisma/prisma/issues/7726
        }
      })
    } else {
      if (errors && errors.length === 1) {
        throw new Error(errors[0].error)
      }
      throw new Error(JSON.stringify(data))
    }
  }

  private buildQueryError(error: unknown) {
    // @ts-ignore
    if (error.user_facing_error.is_panic) {
      console.info('PANIC')
      return new Error(
        // @ts-ignore
        this.getErrorMessageWithLink(error.user_facing_error.message),
      )
    }

    return error
  }
}
