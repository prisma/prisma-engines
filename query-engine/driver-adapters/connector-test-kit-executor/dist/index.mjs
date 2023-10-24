// src/qe.ts
import * as os from "os";
import * as path from "path";
function initQueryEngine(adapter, datamodel, queryLogCallback, debug2) {
  const libExt = os.platform() === "darwin" ? "dylib" : "so";
  const dirname2 = path.dirname(new URL(import.meta.url).pathname);
  const libQueryEnginePath = path.join(dirname2, `../../../../../target/debug/libquery_engine.${libExt}`);
  const libqueryEngine = { exports: {} };
  process.dlopen(libqueryEngine, libQueryEnginePath);
  const QueryEngine = libqueryEngine.exports.QueryEngine;
  const queryEngineOptions = {
    datamodel,
    configDir: ".",
    engineProtocol: "json",
    logLevel: process.env["RUST_LOG"] ?? "info",
    logQueries: true,
    env: process.env,
    ignoreEnvVarErrors: false
  };
  const logCallback = (event) => {
    const parsed = JSON.parse(event);
    if (parsed.is_query) {
      queryLogCallback(parsed.query);
    }
    debug2(parsed);
  };
  return new QueryEngine(queryEngineOptions, logCallback, adapter);
}

// src/index.ts
import * as readline from "readline";
import pgDriver from "pg";
import * as prismaPg from "@prisma/adapter-pg";
import { Pool as NeonPool, neonConfig } from "@neondatabase/serverless";
import { fetch, WebSocket } from "undici";
import * as prismaNeon from "@prisma/adapter-neon";
import { createClient } from "@libsql/client";
import { PrismaLibSQL } from "@prisma/adapter-libsql";
import { connect as planetscaleConnect } from "@planetscale/database";
import { PrismaPlanetScale } from "@prisma/adapter-planetscale";
import { bindAdapter } from "@prisma/driver-adapter-utils";
var SUPPORTED_ADAPTERS = {
  "pg": pgAdapter,
  "neon:ws": neonWsAdapter,
  "libsql": libsqlAdapter,
  "planetscale": planetscaleAdapter
};
var debug = (() => {
  if ((process.env.LOG_LEVEL ?? "").toLowerCase() != "debug") {
    return (...args) => {
    };
  }
  return (...args) => {
    console.error("[nodejs] DEBUG:", ...args);
  };
})();
var err = (...args) => console.error("[nodejs] ERROR:", ...args);
async function main() {
  const iface = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
    terminal: false
  });
  iface.on("line", async (line) => {
    try {
      const request = JSON.parse(line);
      debug(`Got a request: ${line}`);
      try {
        const response = await handleRequest(request.method, request.params);
        respondOk(request.id, response);
      } catch (err2) {
        debug("[nodejs] Error from request handler: ", err2);
        respondErr(request.id, {
          code: 1,
          message: err2.toString()
        });
      }
    } catch (err2) {
      debug("Received non-json line: ", line);
    }
  });
}
var state = {};
async function handleRequest(method, params) {
  switch (method) {
    case "initializeSchema": {
      const castParams = params;
      const logs = [];
      const [engine, adapter] = await initQe(castParams.url, castParams.schema, (log) => {
        logs.push(log);
      });
      await engine.connect("");
      state[castParams.schemaId] = {
        engine,
        adapter,
        logs
      };
      return null;
    }
    case "query": {
      debug("Got `query`", params);
      const castParams = params;
      const engine = state[castParams.schemaId].engine;
      const result = await engine.query(JSON.stringify(castParams.query), "", castParams.txId);
      const parsedResult = JSON.parse(result);
      if (parsedResult.errors) {
        const error = parsedResult.errors[0]?.user_facing_error;
        if (error.error_code === "P2036") {
          const jsError = state[castParams.schemaId].adapter.errorRegistry.consumeError(error.meta.id);
          if (!jsError) {
            err(`Something went wrong. Engine reported external error with id ${error.meta.id}, but it was not registered.`);
          } else {
            err("got error response from the engine caused by the driver: ", jsError);
          }
        }
      }
      debug("got response from engine: ", result);
      return result;
    }
    case "startTx": {
      debug("Got `startTx", params);
      const { schemaId, options } = params;
      const result = await state[schemaId].engine.startTransaction(JSON.stringify(options), "");
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
      const result = await state[schemaId].engine.rollbackTransaction(txId, "{}");
      return JSON.parse(result);
    }
    case "teardown": {
      debug("Got `teardown", params);
      const castParams = params;
      await state[castParams.schemaId].engine.disconnect("");
      delete state[castParams.schemaId];
      return {};
    }
    case "getLogs": {
      const castParams = params;
      return state[castParams.schemaId].logs;
    }
    default: {
      throw new Error(`Unknown method: \`${method}\``);
    }
  }
}
function respondErr(requestId, error) {
  const msg = {
    jsonrpc: "2.0",
    id: requestId,
    error
  };
  console.log(JSON.stringify(msg));
}
function respondOk(requestId, payload) {
  const msg = {
    jsonrpc: "2.0",
    id: requestId,
    result: payload
  };
  console.log(JSON.stringify(msg));
}
async function initQe(url, prismaSchema, logCallback) {
  const adapter = await adapterFromEnv(url);
  const errorCapturingAdapter = bindAdapter(adapter);
  const engineInstance = initQueryEngine(errorCapturingAdapter, prismaSchema, logCallback, debug);
  return [engineInstance, errorCapturingAdapter];
}
async function adapterFromEnv(url) {
  const adapter = process.env.DRIVER_ADAPTER ?? "";
  if (adapter == "") {
    throw new Error("DRIVER_ADAPTER is not defined or empty.");
  }
  if (!(adapter in SUPPORTED_ADAPTERS)) {
    throw new Error(`Unsupported driver adapter: ${adapter}`);
  }
  return await SUPPORTED_ADAPTERS[adapter](url);
}
function postgres_options(url) {
  let args = { connectionString: url };
  const schemaName = new URL(url).searchParams.get("schema");
  if (schemaName != null) {
    args.options = `--search_path="${schemaName}"`;
  }
  return args;
}
async function pgAdapter(url) {
  const pool = new pgDriver.Pool(postgres_options(url));
  return new prismaPg.PrismaPg(pool);
}
async function neonWsAdapter(url) {
  const proxyURL = JSON.parse(process.env.DRIVER_ADAPTER_CONFIG || "{}").proxyUrl ?? "";
  if (proxyURL == "") {
    throw new Error("DRIVER_ADAPTER_CONFIG is not defined or empty, but its required for neon adapter.");
  }
  neonConfig.wsProxy = () => proxyURL;
  neonConfig.webSocketConstructor = WebSocket;
  neonConfig.useSecureWebSocket = false;
  neonConfig.pipelineConnect = false;
  const pool = new NeonPool(postgres_options(url));
  return new prismaNeon.PrismaNeon(pool);
}
async function libsqlAdapter(url) {
  const libsql = createClient({ url, intMode: "bigint" });
  return new PrismaLibSQL(libsql);
}
async function planetscaleAdapter(url) {
  const proxyURL = JSON.parse(process.env.DRIVER_ADAPTER_CONFIG || "{}").proxyUrl ?? "";
  if (proxyURL == "") {
    throw new Error("DRIVER_ADAPTER_CONFIG is not defined or empty, but its required for planetscale adapter.");
  }
  const connection = planetscaleConnect({
    url: proxyURL,
    fetch
  });
  return new PrismaPlanetScale(connection);
}
main().catch(err);
