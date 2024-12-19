import type { DriverAdapter } from "@prisma/driver-adapter-utils";
import { __dirname } from './utils'

export interface SchemaEngine {
  connect(trace: string, requestId: string): Promise<void>;
  disconnect(trace: string, requestId: string): Promise<void>;
  query(body: string, trace: string, tx_id: string | undefined, requestId: string): Promise<string>;
  startTransaction(input: string, trace: string, requestId: string): Promise<string>;
  commitTransaction(tx_id: string, trace: string, requestId: string): Promise<string>;
  rollbackTransaction(tx_id: string, trace: string, requestId: string): Promise<string>;
}

export type QueryLogCallback = (log: string) => void;

export async function initSchemaEngine(
  adapter: DriverAdapter,
  datamodel: string,
  debug: (...args: any[]) => void
): Promise<SchemaEngine> {
  const { getSchemaEngineForProvider: getEngineForProvider } = await import("./schema-engine-wasm");
  const WasmQueryEngine = await getEngineForProvider(adapter.provider)
  return new WasmQueryEngine(adapter);
}
