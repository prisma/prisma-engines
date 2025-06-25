import { ConnectionInfo } from '@prisma/driver-adapter-utils'
import { __dirname } from './utils.js'
import { QueryPlanNode } from '@prisma/client-engine-runtime'

export type QueryCompilerParams = {
  // TODO: support multiple datamodels
  datamodel: string
  provider: 'postgres' | 'mysql' | 'sqlite' | 'sqlserver'
  connectionInfo: ConnectionInfo
}

export interface QueryCompiler {
  new (params: QueryCompilerParams): QueryCompiler
  compile(query: string): QueryPlanNode
  free(): void
}

export async function initQueryCompiler(
  params: QueryCompilerParams,
): Promise<QueryCompiler> {
  const { getQueryCompilerForProvider } = await import(
    './query-compiler-wasm.js'
  )
  const WasmQueryCompiler = (await getQueryCompilerForProvider(
    params.provider,
  )) as QueryCompiler
  return new WasmQueryCompiler(params)
}
