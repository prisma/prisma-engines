use core::panic::PanicInfo;

use alloc::string::ToString;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// This function registers the reason for a Wasm panic via the
    /// JS function `globalThis.PRISMA_WASM_PANIC_REGISTRY.set_message()`
    #[wasm_bindgen(js_namespace = ["global", "PRISMA_WASM_PANIC_REGISTRY"], js_name = "set_message")]
    fn prisma_set_wasm_panic_message(s: &str);
}

#[panic_handler]
fn panic2(info: &PanicInfo) -> ! {
    prisma_set_wasm_panic_message(&info.to_string());
    core::intrinsics::abort()
}
