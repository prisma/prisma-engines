import type { DriverAdapter } from "@prisma/driver-adapter-utils";
import { __dirname } from "./utils";

export type QueryCompilerParams = {
  // TODO: support multiple datamodels
  datamodel: string;
};

export interface QueryCompiler {
  new (params: QueryCompilerParams, adapter: DriverAdapter): QueryCompiler;
  compile(query: string): Promise<string>;
}

export async function initQueryCompiler(
  params: QueryCompilerParams,
  adapter: DriverAdapter,
): Promise<QueryCompiler> {
  const { getQueryCompilerForProvider } = await import("./query-compiler-wasm");
  console.log(getQueryCompilerForProvider);
  const WasmQueryCompiler = (await getQueryCompilerForProvider(
    adapter.provider,
  )) as QueryCompiler;
  return new WasmQueryCompiler(params, adapter);
}
