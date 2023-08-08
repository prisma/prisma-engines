let wasm;
export function __wbg_set_wasm(val) {
    wasm = val;
}


const lTextDecoder = typeof TextDecoder === 'undefined' ? (0, module.require)('util').TextDecoder : TextDecoder;

let cachedTextDecoder = new lTextDecoder('utf-8', { ignoreBOM: true, fatal: true });

cachedTextDecoder.decode();

let cachedUint8Memory0 = null;

function getUint8Memory0() {
    if (cachedUint8Memory0 === null || cachedUint8Memory0.byteLength === 0) {
        cachedUint8Memory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8Memory0;
}

function getStringFromWasm0(ptr, len) {
    return cachedTextDecoder.decode(getUint8Memory0().subarray(ptr, ptr + len));
}

let WASM_VECTOR_LEN = 0;

const lTextEncoder = typeof TextEncoder === 'undefined' ? (0, module.require)('util').TextEncoder : TextEncoder;

let cachedTextEncoder = new lTextEncoder('utf-8');

const encodeString = (typeof cachedTextEncoder.encodeInto === 'function'
    ? function (arg, view) {
    return cachedTextEncoder.encodeInto(arg, view);
}
    : function (arg, view) {
    const buf = cachedTextEncoder.encode(arg);
    view.set(buf);
    return {
        read: arg.length,
        written: buf.length
    };
});

function passStringToWasm0(arg, malloc, realloc) {

    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length);
        getUint8Memory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len);

    const mem = getUint8Memory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }

    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3);
        const view = getUint8Memory0().subarray(ptr + offset, ptr + len);
        const ret = encodeString(arg, view);

        offset += ret.written;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

let cachedInt32Memory0 = null;

function getInt32Memory0() {
    if (cachedInt32Memory0 === null || cachedInt32Memory0.byteLength === 0) {
        cachedInt32Memory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachedInt32Memory0;
}

let cachedFloat64Memory0 = null;

function getFloat64Memory0() {
    if (cachedFloat64Memory0 === null || cachedFloat64Memory0.byteLength === 0) {
        cachedFloat64Memory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachedFloat64Memory0;
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

let cachedBigInt64Memory0 = null;

function getBigInt64Memory0() {
    if (cachedBigInt64Memory0 === null || cachedBigInt64Memory0.byteLength === 0) {
        cachedBigInt64Memory0 = new BigInt64Array(wasm.memory.buffer);
    }
    return cachedBigInt64Memory0;
}

function makeMutClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {
        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            if (--state.cnt === 0) {
                wasm.__wbindgen_export_3.get(state.dtor)(a, state.b);

            } else {
                state.a = a;
            }
        }
    };
    real.original = state;

    return real;
}
function __wbg_adapter_50(arg0, arg1, arg2) {
    wasm.closure343_externref_shim(arg0, arg1, arg2);
}

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
    return instance.ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_export_2.get(idx);
    wasm.__wbindgen_export_5(idx);
    return value;
}
/**
* @returns {any}
*/
export function version() {
    const ret = wasm.version();
    return ret;
}

/**
* @param {string} datamodel_string
* @returns {string}
*/
export function dmmf(datamodel_string) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(datamodel_string, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
        const len0 = WASM_VECTOR_LEN;
        wasm.dmmf(retptr, ptr0, len0);
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        var r2 = getInt32Memory0()[retptr / 4 + 2];
        var r3 = getInt32Memory0()[retptr / 4 + 3];
        var ptr1 = r0;
        var len1 = r1;
        if (r3) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(r2);
        }
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export_6(ptr1, len1);
    }
}

/**
* @param {string | undefined} panic_message
*/
export function debug_panic(panic_message) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        var ptr0 = isLikeNone(panic_message) ? 0 : passStringToWasm0(panic_message, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
        var len0 = WASM_VECTOR_LEN;
        wasm.debug_panic(retptr, ptr0, len0);
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        if (r1) {
            throw takeFromExternrefTable0(r0);
        }
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
*/
export function initPanicHook() {
    wasm.initPanicHook();
}

function addToExternrefTable0(obj) {
    const idx = wasm.__wbindgen_export_8();
    wasm.__wbindgen_export_2.set(idx, obj);
    return idx;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_export_7(idx);
    }
}
function __wbg_adapter_136(arg0, arg1, arg2, arg3) {
    wasm.closure101_externref_shim(arg0, arg1, arg2, arg3);
}

/**
* Proxy is a struct wrapping a javascript object that exhibits basic primitives for
* querying and executing SQL (i.e. a client connector). The Proxy uses sys::Function to
* invoke the code within the node runtime that implements the client connector.
*/
export class Proxy {

    static __wrap(ptr) {
        const obj = Object.create(Proxy.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_proxy_free(ptr);
    }
    /**
    * @param {object} js_connector
    */
    constructor(js_connector) {
        const ret = wasm.proxy_new(js_connector);
        return Proxy.__wrap(ret);
    }
}
/**
* The main query engine used by JS
*/
export class QueryEngine {

    static __wrap(ptr) {
        const obj = Object.create(QueryEngine.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_queryengine_free(ptr);
    }
    /**
    * Parse a validated datamodel and configuration to allow connecting later on.
    * @param {any} options
    * @param {Function} callback
    * @param {Proxy | undefined} maybe_driver
    */
    constructor(options, callback, maybe_driver) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            let ptr0 = 0;
            if (!isLikeNone(maybe_driver)) {
                _assertClass(maybe_driver, Proxy);
                ptr0 = maybe_driver.__destroy_into_raw();
            }
            wasm.queryengine_new(retptr, options, callback, ptr0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeFromExternrefTable0(r1);
            }
            return QueryEngine.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * Connect to the database, allow queries to be run.
    * @param {string} trace
    * @returns {Promise<void>}
    */
    connect(trace) {
        const ptr0 = passStringToWasm0(trace, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.queryengine_connect(this.ptr, ptr0, len0);
        return ret;
    }
    /**
    * Disconnect and drop the core. Can be reconnected later with `#connect`.
    * @param {string} trace
    * @returns {Promise<void>}
    */
    disconnect(trace) {
        const ptr0 = passStringToWasm0(trace, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.queryengine_disconnect(this.ptr, ptr0, len0);
        return ret;
    }
    /**
    * If connected, sends a query to the core and returns the response.
    * @param {string} body
    * @param {string} trace
    * @param {string | undefined} tx_id
    * @returns {Promise<string>}
    */
    query(body, trace, tx_id) {
        const ptr0 = passStringToWasm0(body, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(trace, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
        const len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(tx_id) ? 0 : passStringToWasm0(tx_id, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.queryengine_query(this.ptr, ptr0, len0, ptr1, len1, ptr2, len2);
        return ret;
    }
    /**
    * If connected, attempts to start a transaction in the core and returns its ID.
    * @param {string} input
    * @param {string} trace
    * @returns {Promise<string>}
    */
    startTransaction(input, trace) {
        const ptr0 = passStringToWasm0(input, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(trace, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.queryengine_startTransaction(this.ptr, ptr0, len0, ptr1, len1);
        return ret;
    }
    /**
    * If connected, attempts to commit a transaction with id `tx_id` in the core.
    * @param {string} tx_id
    * @param {string} _trace
    * @returns {Promise<string>}
    */
    commitTransaction(tx_id, _trace) {
        const ptr0 = passStringToWasm0(tx_id, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(_trace, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.queryengine_commitTransaction(this.ptr, ptr0, len0, ptr1, len1);
        return ret;
    }
    /**
    * @param {string} trace
    * @returns {Promise<string>}
    */
    dmmf(trace) {
        const ptr0 = passStringToWasm0(trace, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.queryengine_dmmf(this.ptr, ptr0, len0);
        return ret;
    }
    /**
    * If connected, attempts to roll back a transaction with id `tx_id` in the core.
    * @param {string} tx_id
    * @param {string} _trace
    * @returns {Promise<string>}
    */
    rollbackTransaction(tx_id, _trace) {
        const ptr0 = passStringToWasm0(tx_id, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(_trace, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.queryengine_rollbackTransaction(this.ptr, ptr0, len0, ptr1, len1);
        return ret;
    }
    /**
    * Loads the query schema. Only available when connected.
    * @returns {Promise<string>}
    */
    sdlSchema() {
        const ret = wasm.queryengine_sdlSchema(this.ptr);
        return ret;
    }
}

export function __wbg_now_931686b195a14f9d() {
    const ret = Date.now();
    return ret;
};

export function __wbg_call_9495de66fdbe016b() { return handleError(function (arg0, arg1, arg2) {
    const ret = arg0.call(arg1, arg2);
    return ret;
}, arguments) };

export function __wbindgen_error_new(arg0, arg1) {
    const ret = new Error(getStringFromWasm0(arg0, arg1));
    return ret;
};

export function __wbindgen_string_new(arg0, arg1) {
    const ret = getStringFromWasm0(arg0, arg1);
    return ret;
};

export function __wbg_set_841ac57cff3d672b(arg0, arg1, arg2) {
    arg0[arg1] = arg2;
};

export function __wbindgen_is_object(arg0) {
    const val = arg0;
    const ret = typeof(val) === 'object' && val !== null;
    return ret;
};

export function __wbg_getwithrefkey_5e6d9547403deab8(arg0, arg1) {
    const ret = arg0[arg1];
    return ret;
};

export function __wbindgen_is_undefined(arg0) {
    const ret = arg0 === undefined;
    return ret;
};

export function __wbindgen_in(arg0, arg1) {
    const ret = arg0 in arg1;
    return ret;
};

export function __wbg_length_e498fbc24f9c1d4f(arg0) {
    const ret = arg0.length;
    return ret;
};

export function __wbindgen_string_get(arg0, arg1) {
    const obj = arg1;
    const ret = typeof(obj) === 'string' ? obj : undefined;
    var ptr0 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
    var len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};

export function __wbindgen_jsval_loose_eq(arg0, arg1) {
    const ret = arg0 == arg1;
    return ret;
};

export function __wbindgen_is_string(arg0) {
    const ret = typeof(arg0) === 'string';
    return ret;
};

export function __wbg_get_27fe3dac1c4d0224(arg0, arg1) {
    const ret = arg0[arg1 >>> 0];
    return ret;
};

export function __wbg_new_9d3a9ce4282a18a8(arg0, arg1) {
    try {
        var state0 = {a: arg0, b: arg1};
        var cb0 = (arg0, arg1) => {
            const a = state0.a;
            state0.a = 0;
            try {
                return __wbg_adapter_136(a, state0.b, arg0, arg1);
            } finally {
                state0.a = a;
            }
        };
        const ret = new Promise(cb0);
        return ret;
    } finally {
        state0.a = state0.b = 0;
    }
};

export function __wbg_new_f9876326328f45ed() {
    const ret = new Object();
    return ret;
};

export function __wbg_new0_25059e40b1c02766() {
    const ret = new Date();
    return ret;
};

export function __wbg_getTime_7c59072d1651a3cf(arg0) {
    const ret = arg0.getTime();
    return ret;
};

export function __wbg_new_abda76e883ba8a5f() {
    const ret = new Error();
    return ret;
};

export function __wbg_stack_658279fe44541cf6(arg0, arg1) {
    const ret = arg1.stack;
    const ptr0 = passStringToWasm0(ret, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
    const len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};

export function __wbg_error_f851667af71bcfc6(arg0, arg1) {
    try {
        console.error(getStringFromWasm0(arg0, arg1));
    } finally {
        wasm.__wbindgen_export_6(arg0, arg1);
    }
};

export function __wbindgen_memory() {
    const ret = wasm.memory;
    return ret;
};

export function __wbg_buffer_cf65c07de34b9a08(arg0) {
    const ret = arg0.buffer;
    return ret;
};

export function __wbg_newwithbyteoffsetandlength_9fb2f11355ecadf5(arg0, arg1, arg2) {
    const ret = new Uint8Array(arg0, arg1 >>> 0, arg2 >>> 0);
    return ret;
};

export function __wbg_randomFillSync_dc1e9a60c158336d() { return handleError(function (arg0, arg1) {
    arg0.randomFillSync(arg1);
}, arguments) };

export function __wbg_subarray_7526649b91a252a6(arg0, arg1, arg2) {
    const ret = arg0.subarray(arg1 >>> 0, arg2 >>> 0);
    return ret;
};

export function __wbg_getRandomValues_37fa2ca9e4e07fab() { return handleError(function (arg0, arg1) {
    arg0.getRandomValues(arg1);
}, arguments) };

export function __wbg_crypto_c48a774b022d20ac(arg0) {
    const ret = arg0.crypto;
    return ret;
};

export function __wbg_process_298734cf255a885d(arg0) {
    const ret = arg0.process;
    return ret;
};

export function __wbg_versions_e2e78e134e3e5d01(arg0) {
    const ret = arg0.versions;
    return ret;
};

export function __wbg_node_1cd7a5d853dbea79(arg0) {
    const ret = arg0.node;
    return ret;
};

export function __wbg_require_8f08ceecec0f4fee() { return handleError(function () {
    const ret = module.require;
    return ret;
}, arguments) };

export function __wbindgen_is_function(arg0) {
    const ret = typeof(arg0) === 'function';
    return ret;
};

export function __wbg_msCrypto_bcb970640f50a1e8(arg0) {
    const ret = arg0.msCrypto;
    return ret;
};

export function __wbg_newwithlength_b56c882b57805732(arg0) {
    const ret = new Uint8Array(arg0 >>> 0);
    return ret;
};

export function __wbg_next_88560ec06a094dea() { return handleError(function (arg0) {
    const ret = arg0.next();
    return ret;
}, arguments) };

export function __wbg_done_1ebec03bbd919843(arg0) {
    const ret = arg0.done;
    return ret;
};

export function __wbg_value_6ac8da5cc5b3efda(arg0) {
    const ret = arg0.value;
    return ret;
};

export function __wbg_iterator_55f114446221aa5a() {
    const ret = Symbol.iterator;
    return ret;
};

export function __wbg_get_baf4855f9a986186() { return handleError(function (arg0, arg1) {
    const ret = Reflect.get(arg0, arg1);
    return ret;
}, arguments) };

export function __wbg_call_95d1ea488d03e4e8() { return handleError(function (arg0, arg1) {
    const ret = arg0.call(arg1);
    return ret;
}, arguments) };

export function __wbg_next_b7d530c04fd8b217(arg0) {
    const ret = arg0.next;
    return ret;
};

export function __wbg_self_e7c1f827057f6584() { return handleError(function () {
    const ret = self.self;
    return ret;
}, arguments) };

export function __wbg_window_a09ec664e14b1b81() { return handleError(function () {
    const ret = window.window;
    return ret;
}, arguments) };

export function __wbg_globalThis_87cbb8506fecf3a9() { return handleError(function () {
    const ret = globalThis.globalThis;
    return ret;
}, arguments) };

export function __wbg_global_c85a9259e621f3db() { return handleError(function () {
    const ret = global.global;
    return ret;
}, arguments) };

export function __wbg_newnoargs_2b8b6bd7753c76ba(arg0, arg1) {
    const ret = new Function(getStringFromWasm0(arg0, arg1));
    return ret;
};

export function __wbg_new_537b7341ce90bb31(arg0) {
    const ret = new Uint8Array(arg0);
    return ret;
};

export function __wbg_set_17499e8aa4003ebd(arg0, arg1, arg2) {
    arg0.set(arg1, arg2 >>> 0);
};

export function __wbg_length_27a2afe8ab42b09f(arg0) {
    const ret = arg0.length;
    return ret;
};

export function __wbindgen_boolean_get(arg0) {
    const v = arg0;
    const ret = typeof(v) === 'boolean' ? (v ? 1 : 0) : 2;
    return ret;
};

export function __wbindgen_number_get(arg0, arg1) {
    const obj = arg1;
    const ret = typeof(obj) === 'number' ? obj : undefined;
    getFloat64Memory0()[arg0 / 8 + 1] = isLikeNone(ret) ? 0 : ret;
    getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
};

export function __wbg_instanceof_Uint8Array_01cebe79ca606cca(arg0) {
    let result;
    try {
        result = arg0 instanceof Uint8Array;
    } catch {
        result = false;
    }
    const ret = result;
    return ret;
};

export function __wbg_instanceof_ArrayBuffer_a69f02ee4c4f5065(arg0) {
    let result;
    try {
        result = arg0 instanceof ArrayBuffer;
    } catch {
        result = false;
    }
    const ret = result;
    return ret;
};

export function __wbg_entries_4e1315b774245952(arg0) {
    const ret = Object.entries(arg0);
    return ret;
};

export function __wbg_isSafeInteger_8c4789029e885159(arg0) {
    const ret = Number.isSafeInteger(arg0);
    return ret;
};

export function __wbg_String_88810dfeb4021902(arg0, arg1) {
    const ret = String(arg1);
    const ptr0 = passStringToWasm0(ret, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
    const len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};

export function __wbindgen_debug_string(arg0, arg1) {
    const ret = debugString(arg1);
    const ptr0 = passStringToWasm0(ret, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
    const len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};

export function __wbindgen_throw(arg0, arg1) {
    throw new Error(getStringFromWasm0(arg0, arg1));
};

export function __wbindgen_cb_drop(arg0) {
    const obj = arg0.original;
    if (obj.cnt-- == 1) {
        obj.a = 0;
        return true;
    }
    const ret = false;
    return ret;
};

export function __wbg_then_ec5db6d509eb475f(arg0, arg1) {
    const ret = arg0.then(arg1);
    return ret;
};

export function __wbg_then_f753623316e2873a(arg0, arg1, arg2) {
    const ret = arg0.then(arg1, arg2);
    return ret;
};

export function __wbg_resolve_fd40f858d9db1a04(arg0) {
    const ret = Promise.resolve(arg0);
    return ret;
};

export function __wbindgen_is_bigint(arg0) {
    const ret = typeof(arg0) === 'bigint';
    return ret;
};

export function __wbindgen_bigint_get_as_i64(arg0, arg1) {
    const v = arg1;
    const ret = typeof(v) === 'bigint' ? v : undefined;
    getBigInt64Memory0()[arg0 / 8 + 1] = isLikeNone(ret) ? BigInt(0) : ret;
    getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
};

export function __wbindgen_bigint_from_i64(arg0) {
    const ret = arg0;
    return ret;
};

export function __wbindgen_jsval_eq(arg0, arg1) {
    const ret = arg0 === arg1;
    return ret;
};

export function __wbindgen_bigint_from_u64(arg0) {
    const ret = BigInt.asUintN(64, arg0);
    return ret;
};

export function __wbg_isArray_39d28997bf6b96b4(arg0) {
    const ret = Array.isArray(arg0);
    return ret;
};

export function __wbg_new_b525de17f44a8943() {
    const ret = new Array();
    return ret;
};

export function __wbg_set_17224bc548dd1d7b(arg0, arg1, arg2) {
    arg0[arg1 >>> 0] = arg2;
};

export function __wbindgen_number_new(arg0) {
    const ret = arg0;
    return ret;
};

export function __wbg_new_f841cc6f2098f4b5() {
    const ret = new Map();
    return ret;
};

export function __wbg_set_388c4c6422704173(arg0, arg1, arg2) {
    const ret = arg0.set(arg1, arg2);
    return ret;
};

export function __wbg_new_15d3966e9981a196(arg0, arg1) {
    const ret = new Error(getStringFromWasm0(arg0, arg1));
    return ret;
};

export function __wbindgen_closure_wrapper5658(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 344, __wbg_adapter_50);
    return ret;
};

export function __wbindgen_init_externref_table() {
    const table = wasm.__wbindgen_export_2;
    const offset = table.grow(4);
    table.set(0, undefined);
    table.set(offset + 0, undefined);
    table.set(offset + 1, null);
    table.set(offset + 2, true);
    table.set(offset + 3, false);
    ;
};

