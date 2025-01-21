import * as wasmPostgres from "../../../../query-compiler/query-compiler-wasm/pkg/postgresql/query_compiler_bg.js";
import * as wasmMysql from "../../../../query-compiler/query-compiler-wasm/pkg/mysql/query_compiler_bg.js";
import * as wasmSqlite from "../../../../query-compiler/query-compiler-wasm/pkg/sqlite/query_compiler_bg.js";
import fs from "node:fs/promises";
import path from "node:path";
import { __dirname } from "./utils.js";

const wasm = {
  postgres: wasmPostgres,
  mysql: wasmMysql,
  sqlite: wasmSqlite,
};

type EngineName = keyof typeof wasm;

const initializedModules = new Set<EngineName>();

export async function getQueryCompilerForProvider(provider: EngineName) {
  const engine = wasm[provider];
  if (!initializedModules.has(provider)) {
    const subDir = provider === "postgres" ? "postgresql" : provider;
    const bytes = await fs.readFile(
      path.resolve(
        __dirname,
        "..",
        "..",
        "..",
        "..",
        "query-compiler",
        "query-compiler-wasm",
        "pkg",
        subDir,
        "query_compiler_bg.wasm",
      ),
    );
    const module = new WebAssembly.Module(bytes);
    const instance = new WebAssembly.Instance(module, {
      "./query_compiler_bg.js": engine,
    });
    engine.__wbg_set_wasm(instance.exports);
    initializedModules.add(provider);
  }

  return engine.QueryCompiler;
}
