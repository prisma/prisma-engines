# `wasm-opt`

## Things to keep in mind

The following `wasm-opt` flags will cause disruption, as they will yield failures at runtime when running the optimized Wasm binary:

- `--remove-memory`: this removes every `data` section, which contains error messages and other static data of the binary.
  ```
  > node --experimental-wasm-modules ./example.js

  wasm://wasm/00b13efa:1


  RuntimeError: null function or function signature mismatch
      at wasm://wasm/00b13efa:wasm-function[915]:0x20132b
      at wasm://wasm/00b13efa:wasm-function[887]:0x1fbe12
      at wasm://wasm/00b13efa:wasm-function[2534]:0x29385f
      at wasm://wasm/00b13efa:wasm-function[88]:0x2b28f
      at new QueryEngine (file:///Users/jkomyno/work/prisma/prisma-engines/query-engine/query-engine-wasm/pkg/query_engine_bg.js:378:18)
      at main (file:///Users/jkomyno/work/prisma/prisma-engines/query-engine/query-engine-wasm/example/example.js:35:23)
  ```
  This option alone reduces the gzipped binary size by 0.2 MB, but sadly we can't use it as things currently stand.


- `--enable-reference-types`: this enables reference types, which are not compatible with how we use `serde` at the moment.
  ```
  > node --experimental-wasm-modules ./example.js

  log-callback
  log-callback
  log-callback
  log-callback
  {
    created: '{"errors":[{"error":"Error in connector: Database error. error code: WASM_ERROR, error message: Error: invalid type: JsValue(undefined), expected unit\\n    at __wbindgen_error_new (file:///Users/jkomyno/work/prisma/prisma-engines/query-engine/query-engine-wasm/pkg/query_engine_bg.js:520:17)\\n    at wasm://wasm/00cd3c02:wasm-function[4473]:0x2c3664\\n    at wasm://wasm/00cd3c02:wasm-function[2468]:0x279d75\\n    at wasm://wasm/00cd3c02:wasm-function[2886]:0x28a509\\n    at wasm://wasm/00cd3c02:wasm-function[1056]:0x1ff3e8\\n    at wasm://wasm/00cd3c02:wasm-function[581]:0x1a278f\\n    at wasm://wasm/00cd3c02:wasm-function[619]:0x1abbb5\\n    at wasm://wasm/00cd3c02:wasm-function[816]:0x1d8cc4\\n    at wasm://wasm/00cd3c02:wasm-function[374]:0x1518b8\\n    at wasm://wasm/00cd3c02:wasm-function[175]:0xbe722","user_facing_error":{"is_panic":false,"message":"Raw query failed. Code: `WASM_ERROR`. Message: `Error: invalid type: JsValue(undefined), expected unit\\n    at __wbindgen_error_new (file:///Users/jkomyno/work/prisma/prisma-engines/query-engine/query-engine-wasm/pkg/query_engine_bg.js:520:17)\\n    at wasm://wasm/00cd3c02:wasm-function[4473]:0x2c3664\\n    at wasm://wasm/00cd3c02:wasm-function[2468]:0x279d75\\n    at wasm://wasm/00cd3c02:wasm-function[2886]:0x28a509\\n    at wasm://wasm/00cd3c02:wasm-function[1056]:0x1ff3e8\\n    at wasm://wasm/00cd3c02:wasm-function[581]:0x1a278f\\n    at wasm://wasm/00cd3c02:wasm-function[619]:0x1abbb5\\n    at wasm://wasm/00cd3c02:wasm-function[816]:0x1d8cc4\\n    at wasm://wasm/00cd3c02:wasm-function[374]:0x1518b8\\n    at wasm://wasm/00cd3c02:wasm-function[175]:0xbe722`","meta":{"code":"WASM_ERROR","message":"Error: invalid type: JsValue(undefined), expected unit\\n    at __wbindgen_error_new (file:///Users/jkomyno/work/prisma/prisma-engines/query-engine/query-engine-wasm/pkg/query_engine_bg.js:520:17)\\n    at wasm://wasm/00cd3c02:wasm-function[4473]:0x2c3664\\n    at wasm://wasm/00cd3c02:wasm-function[2468]:0x279d75\\n    at wasm://wasm/00cd3c02:wasm-function[2886]:0x28a509\\n    at wasm://wasm/00cd3c02:wasm-function[1056]:0x1ff3e8\\n    at wasm://wasm/00cd3c02:wasm-function[581]:0x1a278f\\n    at wasm://wasm/00cd3c02:wasm-function[619]:0x1abbb5\\n    at wasm://wasm/00cd3c02:wasm-function[816]:0x1d8cc4\\n    at wasm://wasm/00cd3c02:wasm-function[374]:0x1518b8\\n    at wasm://wasm/00cd3c02:wasm-function[175]:0xbe722"},"error_code":"P2010"}}]}'
  }
  log-callback
  query result = 
  {
    data: { findManyUser: [ { id: 1235 } ] }
  }
  ```

- `--remove-non-js-ops`: this removes all operations that are not supported by JavaScript, but also results in code packages not found.
  ```
  > node --experimental-wasm-modules ./example.js

  node:internal/errors:497
      ErrorCaptureStackTrace(err);
      ^

  Error [ERR_MODULE_NOT_FOUND]: Cannot find package 'env' imported from /Users/jkomyno/work/prisma/prisma-engines/query-engine/query-engine-wasm/pkg/query_engine_bg.wasm
      at new NodeError (node:internal/errors:406:5)
      at packageResolve (node:internal/modules/esm/resolve:789:9)
      at moduleResolve (node:internal/modules/esm/resolve:838:20)
      at defaultResolve (node:internal/modules/esm/resolve:1043:11)
      at ModuleLoader.defaultResolve (node:internal/modules/esm/loader:383:12)
      at ModuleLoader.resolve (node:internal/modules/esm/loader:352:25)
      at ModuleLoader.getModuleJob (node:internal/modules/esm/loader:228:38)
      at ModuleWrap.<anonymous> (node:internal/modules/esm/module_job:85:39)
      at link (node:internal/modules/esm/module_job:84:36) {
    code: 'ERR_MODULE_NOT_FOUND'
  }
  ```
