use super::timings::TimingsLayer;
use js_sys::Function as JsFunction;
use std::io::{self, Write};
use tracing::Dispatch;
use tracing_error::ErrorLayer;
use tracing_subscriber::{fmt, prelude::*};
use wasm_bindgen::prelude::*;

/// A custom writer that sends log output to a JavaScript callback.
struct JsLogWriter {
    callback: JsFunction,
}

unsafe impl Send for JsLogWriter {}
unsafe impl Sync for JsLogWriter {}

impl Write for JsLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Convert the log message from bytes to a UTF-8 string.
        let s = std::str::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        // Call the JS callback with the log message.
        let _ = self.callback.call1(&JsValue::NULL, &JsValue::from_str(s));
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A MakeWriter implementation that creates a new JsLogWriter.
#[derive(Clone)]
struct JsLogWriterMaker {
    callback: JsFunction,
}

unsafe impl Send for JsLogWriterMaker {}
unsafe impl Sync for JsLogWriterMaker {}

impl JsLogWriterMaker {
    pub fn new(callback: JsFunction) -> Self {
        Self { callback }
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for JsLogWriterMaker {
    type Writer = JsLogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        JsLogWriter {
            callback: self.callback.clone(),
        }
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn error(a: &str);
}

/// Initializes the global logger using the provided JavaScript callback for log output.
pub fn init_logger(log_callback: JsFunction) -> Dispatch {
    let js_writer = JsLogWriterMaker::new(log_callback);

    let subscriber = fmt::Subscriber::builder()
        .json()
        .without_time()
        .with_writer(js_writer)
        .finish()
        .with(ErrorLayer::default())
        .with(TimingsLayer);

    Dispatch::new(subscriber)
}
