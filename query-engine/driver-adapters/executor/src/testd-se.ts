import * as readline from "node:readline";
import * as S from "@effect/schema/Schema";
import {
  bindAdapter,
  ErrorCapturingDriverAdapter,
} from "@prisma/driver-adapter-utils";

import type { DriverAdaptersManager } from "./driver-adapters-manager";
import { jsonRpc, Env } from "./types";
import * as se from "./schema-engine";
import { debug, err } from "./utils"; 
import { setupDriverAdaptersManager } from "./setup";

async function main(): Promise<void> {
  const env = S.decodeUnknownSync(Env)(process.env);
  console.log("[env]", env);

  const iface = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
    terminal: false,
  });

  iface.on("line", async (line) => {
    try {
      const request = S.decodeSync(jsonRpc.RequestFromString)(line);
      debug(`Got a request: ${line}`);

      try {
        const response = await handleRequest(request, env);
        respondOk(request.id, response);
      } catch (err) {
        debug("[nodejs] Error from request handler: ", err);
        respondErr(request.id, {
          code: 1,
          message: err.stack ?? err.toString(),
        });
      }
    } catch (err) {
      debug("Received non-json line: ", line);
      console.error(err);
    }
  });
}

const state: Record<
  number,
  {
    engine: se.SchemaEngine;
    driverAdapterManager: DriverAdaptersManager;
    adapter: ErrorCapturingDriverAdapter | null;
    logs: string[];
  }
> = {};

async function handleRequest(
  { method, params }: jsonRpc.Request,
  env: Env
): Promise<unknown> {
  switch (method) {
    case "initializeSchema": {
      const { url, schema, schemaId, migrationScript } = params;
      const logs = [] as string[];

      const logCallback = (log) => {
        logs.push(log);
      };

      const driverAdapterManager = await setupDriverAdaptersManager(
        env,
        migrationScript
      );

      const { engine, adapter } = await initSe({
        env,
        url,
        driverAdapterManager,
        schema,
      });

      state[schemaId] = {
        engine,
        driverAdapterManager,
        adapter,
        logs,
      };

      if (adapter && adapter.getConnectionInfo) {
        const maxBindValuesResult = adapter.getConnectionInfo().map(info => info.maxBindValues)
        if (maxBindValuesResult.ok) {
          return { maxBindValues: maxBindValuesResult.value }
        }
      }

      return { maxBindValues: null }
    }
    default: {
      throw new Error(`Unknown method: \`${method}\``);
    }
  }
}

function respondErr(requestId: number, error: jsonRpc.RpcError) {
  const msg: jsonRpc.ErrResponse = {
    jsonrpc: "2.0",
    id: requestId,
    error,
  };
  console.log(JSON.stringify(msg));
}

function respondOk(requestId: number, payload: unknown) {
  const msg: jsonRpc.OkResponse = {
    jsonrpc: "2.0",
    id: requestId,
    result: payload,
  };
  console.log(JSON.stringify(msg));
}

type InitSchemaEngineParams = {
  env: Env;
  driverAdapterManager: DriverAdaptersManager;
  url: string;
  schema: string;
};

async function initSe({ env, driverAdapterManager, url, schema }: InitSchemaEngineParams) {
  const adapter = await driverAdapterManager.connect({ url })
  const errorCapturingAdapter = bindAdapter(adapter)
  const engineInstance = await se.initSchemaEngine(
    errorCapturingAdapter,
    schema,
    debug,
  )

  return {
    engine: engineInstance,
    adapter: errorCapturingAdapter,
  }
}

main().catch(err);
