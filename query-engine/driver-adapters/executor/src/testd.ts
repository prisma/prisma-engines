import * as readline from "node:readline";
import { match } from "ts-pattern";
import * as S from "@effect/schema/Schema";
import {
  bindAdapter,
  ErrorCapturingDriverAdapter,
} from "@prisma/driver-adapter-utils";
import { webcrypto } from "node:crypto";

import type { DriverAdaptersManager } from "./driver-adapters-manager";
import { jsonRpc, Env } from "./types";
import * as qe from "./qe";
import { PgManager } from "./driver-adapters-manager/pg";
import { NeonWsManager } from "./driver-adapters-manager/neon.ws";
import { LibSQLManager } from "./driver-adapters-manager/libsql";
import { PlanetScaleManager } from "./driver-adapters-manager/planetscale";
import { D1Manager } from "./driver-adapters-manager/d1";
import { createRNEngineConnector } from "./rn";

if (!global.crypto) {
  global.crypto = webcrypto as Crypto;
}

async function initialiseDriverAdapterManager(
  env: Env,
  migrationScript?: string
): Promise<DriverAdaptersManager> {
  return match(env)
    .with({ DRIVER_ADAPTER: "pg" }, async (env) => await PgManager.setup(env))
    .with(
      { DRIVER_ADAPTER: "neon:ws" },
      async (env) => await NeonWsManager.setup(env)
    )
    .with(
      { DRIVER_ADAPTER: "libsql" },
      async (env) => await LibSQLManager.setup(env)
    )
    .with(
      { DRIVER_ADAPTER: "planetscale" },
      async (env) => await PlanetScaleManager.setup(env)
    )
    .with(
      { DRIVER_ADAPTER: "d1" },
      async (env) => await D1Manager.setup(env, migrationScript)
    )
    .exhaustive();
}

// conditional debug logging based on LOG_LEVEL env var
const debug = (() => {
  if ((process.env.LOG_LEVEL ?? "").toLowerCase() != "debug") {
    return (...args: any[]) => {};
  }

  return (...args: any[]) => {
    console.error("[nodejs] DEBUG:", ...args);
  };
})();

// error logger
const err = (...args: any[]) => console.error("[nodejs] ERROR:", ...args);

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
    engine: qe.QueryEngine;
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

      const driverAdapterManager = await initialiseDriverAdapterManager(
        env,
        migrationScript
      );

      const { engine, adapter } = await initQe({
        env,
        url,
        driverAdapterManager,
        schema,
        logCallback,
      });
      await engine.connect("");

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
    case "query": {
      debug("Got `query`", params);
      const { query, schemaId, txId } = params;
      const engine = state[schemaId].engine;
      const result = await engine.query(JSON.stringify(query), "", txId);

      const parsedResult = JSON.parse(result);
      if (parsedResult.errors) {
        const error = parsedResult.errors[0]?.user_facing_error;
        if (error.error_code === "P2036") {
          const jsError = state[schemaId].adapter?.errorRegistry.consumeError(
            error.meta.id
          );
          if (!jsError) {
            err(
              `Something went wrong. Engine reported external error with id ${error.meta.id}, but it was not registered.`
            );
          } else {
            err(
              "got error response from the engine caused by the driver: ",
              jsError
            );
          }
        }
      }

      debug("🟢 Engine response: ", result);
      // returning unparsed string: otherwise, some information gots lost during this round-trip.
      // In particular, floating point without decimal part turn into integers
      return result;
    }

    case "startTx": {
      debug("Got `startTx", params);
      const { schemaId, options } = params;
      const result = await state[schemaId].engine.startTransaction(
        JSON.stringify(options),
        ""
      );
      return JSON.parse(result);
    }

    case "commitTx": {
      debug("Got `commitTx", params);
      const { schemaId, txId } = params;
      const result = await state[schemaId].engine.commitTransaction(txId, "{}");
      return JSON.parse(result);
    }

    case "rollbackTx": {
      debug("Got `rollbackTx", params);
      const { schemaId, txId } = params;
      const result = await state[schemaId].engine.rollbackTransaction(
        txId,
        "{}"
      );
      return JSON.parse(result);
    }
    case "teardown": {
      debug("Got `teardown", params);
      const { schemaId } = params;

      await state[schemaId].engine.disconnect("");
      await state[schemaId].driverAdapterManager.teardown();
      delete state[schemaId];

      return {};
    }
    case "getLogs": {
      const { schemaId } = params;
      return state[schemaId].logs;
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

type InitQueryEngineParams = {
  env: Env
  driverAdapterManager: DriverAdaptersManager
  url: string
  schema: string
  logCallback: qe.QueryLogCallback
};

async function initQe({
  env,
  driverAdapterManager,
  url,
  schema,
  logCallback,
}: InitQueryEngineParams) {
  if (env.EXTERNAL_TEST_EXECUTOR === 'Mobile') {
    url = env.MOBILE_EMULATOR_URL;

    const engine = createRNEngineConnector(url, schema, logCallback);
    return { engine, adapter: null };
  }

  const adapter = await driverAdapterManager.connect({ url })
  const errorCapturingAdapter = bindAdapter(adapter)
  const engineInstance = await qe.initQueryEngine(
    env.EXTERNAL_TEST_EXECUTOR,
    errorCapturingAdapter,
    schema,
    logCallback,
    debug
  )

  return {
    engine: engineInstance,
    adapter: errorCapturingAdapter,
  }
}

main().catch(err);
