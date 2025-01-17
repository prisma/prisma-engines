import { ConnectionInfo } from "@prisma/driver-adapter-utils";
import { __dirname } from "./utils";

export type QueryCompilerParams = {
  // TODO: support multiple datamodels
  datamodel: string;
  flavour: 'postgres' | 'mysql' | 'sqlite';
  connectionInfo: ConnectionInfo;
};

export interface QueryCompiler {
  new (params: QueryCompilerParams): QueryCompiler;
  compile(query: string): Promise<string>;
}

export async function initQueryCompiler(
  params: QueryCompilerParams,
): Promise<QueryCompiler> {
  const { getQueryCompilerForProvider } = await import("./query-compiler-wasm");
  const WasmQueryCompiler = (await getQueryCompilerForProvider(
    params.flavour,
  )) as QueryCompiler;
  return new WasmQueryCompiler(params);
}
