/**
 * Run with: `node --experimental-wasm-modules ./example.js`
 * on Node.js 18+.
 */
import { webcrypto } from "node:crypto";
import * as fs from "node:fs/promises";
import path from "node:path";
import { __dirname } from './utils'

import * as qe from "./qe";

import { pg } from "@prisma/bundled-js-drivers";
import * as prismaPg from "@prisma/adapter-pg";
import { bindAdapter, DriverAdapter } from "@prisma/driver-adapter-utils";

import { recording } from "./recording";
import prismaQueries from "../bench/queries.json";

import { run, bench, group, baseline } from "mitata";

import { QueryEngine as WasmBaseline } from "query-engine-wasm-baseline";

// `query-engine-wasm-latest` refers to the latest published version of the Wasm Query Engine,
// rather than the latest locally built one. We're pulling in the Postgres Query Engine
// because benchmarks are only run against a Postgres database.
import { QueryEngine as WasmLatest } from "query-engine-wasm-latest/postgresql/query_engine.js";

if (!global.crypto) {
  (global as any).crypto = webcrypto;
}

async function main(): Promise<void> {
  // read the prisma schema from stdin

  var datamodel = (
    await fs.readFile(path.resolve(__dirname, "..", "bench", "schema.prisma"))
  ).toString();

  const url = process.env.DATABASE_URL;
  if (url == null) {
    throw new Error("DATABASE_URL is not defined");
  }
  const pg = await pgAdapter(url);
  const withErrorCapturing = bindAdapter(pg);

  // We build two decorators for recording and replaying db queries.
  const { recorder, replayer, recordings } = recording(withErrorCapturing);

  // We exercise the queries recording them
  await recordQueries(recorder, datamodel, prismaQueries);

  // Dump recordings if requested
  if (process.env.BENCH_RECORDINGS_FILE != null) {
    const recordingsJson = JSON.stringify(recordings.data(), null, 2);
    await fs.writeFile(process.env.BENCH_RECORDINGS_FILE, recordingsJson);
    debug(`Recordings written to ${process.env.BENCH_RECORDINGS_FILE}`);
  }

  // Then we benchmark the execution of the queries but instead of hitting the DB
  // we fetch results from the recordings, thus isolating the performance
  // of the engine + driver adapter code from that of the DB IO.
  await benchMarkQueries(replayer, datamodel, prismaQueries);
}

async function recordQueries(
  adapter: DriverAdapter,
  datamodel: string,
  prismaQueries: any
): Promise<void> {
  // Different engines might have made different SQL queries to complete the same Prisma Query,
  // so we record the results of all engines for the benchmarking phase.
  const napi = await initQeNapiCurrent(adapter, datamodel);
  await napi.connect("");
  const wasmCurrent = await initQeWasmCurrent(adapter, datamodel);
  await wasmCurrent.connect("");
  const wasmBaseline = await initQeWasmBaseLine(adapter, datamodel);
  await wasmBaseline.connect("");
  const wasmLatest = await initQeWasmLatest(adapter, datamodel);
  await wasmLatest.connect("");

  try {
    for (const qe of [napi, wasmCurrent, wasmBaseline, wasmLatest]) {
      for (const prismaQuery of prismaQueries) {
        const { description, query } = prismaQuery;
        const res = await qe.query(JSON.stringify(query), "", undefined);
        console.log(res[9]);

        const errors = JSON.parse(res).errors;
        if (errors != null) {
          throw new Error(
            `Query failed for ${description}: ${JSON.stringify(res)}`
          );
        }
      }
    }
  } finally {
    await napi.disconnect("");
    await wasmCurrent.disconnect("");
    await wasmBaseline.disconnect("");
    await wasmLatest.disconnect("");
  }
}

async function benchMarkQueries(
  adapter: DriverAdapter,
  datamodel: string,
  prismaQueries: any
) {
  const napi = await initQeNapiCurrent(adapter, datamodel);
  await napi.connect("");
  const wasmCurrent = await initQeWasmCurrent(adapter, datamodel);
  await wasmCurrent.connect("");
  const wasmBaseline = await initQeWasmBaseLine(adapter, datamodel);
  await wasmBaseline.connect("");
  const wasmLatest = await initQeWasmLatest(adapter, datamodel);
  await wasmLatest.connect("");

  for (const prismaQuery of prismaQueries) {
    const { description, query } = prismaQuery;

    const engines = {
      Napi: napi,
      "WASM Current": wasmCurrent,
      "WASM Baseline": wasmBaseline,
      "WASM Latest": wasmLatest,
    };

    for (const [engineName, engine] of Object.entries(engines)) {
      const res = await engine.query(JSON.stringify(query), "", undefined);
      const errors = JSON.parse(res).errors;
      if (errors != null && errors.length > 0) {
        throw new Error(
          `${engineName} - Query failed for ${description}: ${JSON.stringify(
            res
          )}`
        );
      }
    }
  }

  try {
    for (const prismaQuery of prismaQueries) {
      const { description, query } = prismaQuery;
      const jsonQuery = JSON.stringify(query);
      const irrelevantTraceId = "";
      const noTx = undefined;

      group(description, () => {
        bench(
          "Web Assembly: Baseline",
          async () =>
            await wasmBaseline.query(jsonQuery, irrelevantTraceId, noTx)
        );
        bench(
          "Web Assembly: Latest",
          async () => await wasmLatest.query(jsonQuery, irrelevantTraceId, noTx)
        );
        baseline(
          "Web Assembly: Current",
          async () =>
            await wasmCurrent.query(jsonQuery, irrelevantTraceId, noTx)
        );
        bench(
          "Node API: Current",
          async () => await napi.query(jsonQuery, irrelevantTraceId, noTx)
        );
      });
    }

    await run({
      colors: false,
      collect: true,
    });
  } finally {
    await napi.disconnect("");
    await wasmCurrent.disconnect("");
    await wasmBaseline.disconnect("");
    await wasmLatest.disconnect("");
  }
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

async function pgAdapter(url: string): Promise<DriverAdapter> {
  const schemaName = new URL(url).searchParams.get("schema") ?? undefined;
  let args: any = { connectionString: url };
  if (schemaName != null) {
    args.options = `--search_path="${schemaName}"`;
  }
  const pool = new pg.Pool(args);

  return new prismaPg.PrismaPg(pool, {
    schema: schemaName,
  });
}

async function initQeNapiCurrent(
  adapter: DriverAdapter,
  datamodel: string
): Promise<qe.QueryEngine> {
  return await qe.initQueryEngine("Napi", adapter, datamodel, debug, debug);
}

async function initQeWasmCurrent(
  adapter: DriverAdapter,
  datamodel: string
): Promise<qe.QueryEngine> {
  return await qe.initQueryEngine(
    "Wasm",
    adapter,
    datamodel,
    (...args) => {},
    debug
  );
}

async function initQeWasmLatest(
  adapter: DriverAdapter,
  datamodel: string
): Promise<qe.QueryEngine> {
  return new WasmLatest(qe.queryEngineOptions(datamodel), debug, adapter);
}

function initQeWasmBaseLine(
  adapter: DriverAdapter,
  datamodel: string
): qe.QueryEngine {
  return new WasmBaseline(qe.queryEngineOptions(datamodel), debug, adapter);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
