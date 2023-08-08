import * as wasm from "./libquery_wasm_bg.wasm";
import { __wbg_set_wasm } from "./libquery_wasm_bg.js";
__wbg_set_wasm(wasm);
export * from "./libquery_wasm_bg.js";

wasm.__wbindgen_start();
