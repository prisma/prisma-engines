import { ConnectionInfo } from '@prisma/driver-adapter-utils'
import { __dirname } from './utils.js'
import { QueryPlanNode } from '@prisma/client-engine-runtime'
import { Env } from './types/index.js'

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
  connector: Env['CONNECTOR'],
): Promise<QueryCompiler> {
  const { getQueryCompilerForConnector } = await import(
    './query-compiler-wasm.js'
  )
  const WasmQueryCompiler = (await getQueryCompilerForConnector(
    connector,
  )) as QueryCompiler
  return new WasmQueryCompiler(params)
}
