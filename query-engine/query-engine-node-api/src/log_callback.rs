use std::os::raw::c_void;
use std::sync::{Arc, Mutex};

use napi::sys::{
    napi_call_threadsafe_function, napi_create_threadsafe_function, napi_env, napi_release_threadsafe_function,
    napi_threadsafe_function, napi_unref_threadsafe_function, napi_value, Status, ThreadsafeFunctionCallMode,
    ThreadsafeFunctionReleaseMode,
};
use napi::{Env, JsFunction, NapiRaw, NapiValue};

enum ThreadsafeFunctionState {
    Active(napi_threadsafe_function),
    Closing,
}

impl ThreadsafeFunctionState {
    fn as_active(&self) -> napi::Result<napi_threadsafe_function> {
        match self {
            ThreadsafeFunctionState::Active(tsfn) => Ok(*tsfn),
            ThreadsafeFunctionState::Closing => Err(closing_error()),
        }
    }
}

pub struct LogCallback {
    state: Arc<Mutex<ThreadsafeFunctionState>>,
}

impl LogCallback {
    pub fn new(env: Env, function: JsFunction) -> napi::Result<Self> {
        let name = env.create_string("prisma log callback")?;
        let state = Arc::new(Mutex::new(ThreadsafeFunctionState::Closing));
        let mut tsfn = std::ptr::null_mut();
        let status = unsafe {
            napi_create_threadsafe_function(
                env.raw(),
                function.raw(),
                std::ptr::null_mut(),
                name.raw(),
                0,
                1,
                Arc::into_raw(Arc::clone(&state)) as *mut c_void,
                Some(finalize_callback),
                std::ptr::null_mut(),
                Some(call_js),
                &mut tsfn,
            )
        };

        match status {
            Status::napi_ok => {
                *(state.lock().unwrap()) = ThreadsafeFunctionState::Active(tsfn);
                Ok(Self { state })
            }

            _ => Err(napi::Error::new(
                napi::Status::from(status),
                "could not create threadsafe function".into(),
            )),
        }
    }

    pub fn unref(&self, env: &Env) -> napi::Result<()> {
        let mut tsfn_state = self.state.lock().unwrap();
        let tsfn = tsfn_state.as_active()?;

        let status = unsafe { napi_unref_threadsafe_function(env.raw(), tsfn) };

        check_status_and_close(status, &mut tsfn_state)
    }

    pub fn call(&self, value: String) -> napi::Result<()> {
        let mut tsfn_state = self.state.lock().unwrap();

        let tsfn = tsfn_state.as_active()?;

        let data = Box::into_raw(Box::new(value));
        let status = unsafe { napi_call_threadsafe_function(tsfn, data.cast(), ThreadsafeFunctionCallMode::blocking) };

        check_status_and_close(status, &mut tsfn_state)
    }
}

impl Drop for LogCallback {
    fn drop(&mut self) {
        let state = self.state.lock().unwrap();
        if let ThreadsafeFunctionState::Active(tsfn) = &*state {
            unsafe {
                napi_release_threadsafe_function(*tsfn, ThreadsafeFunctionReleaseMode::release);
            }
        }
    }
}

unsafe impl Send for LogCallback {}
unsafe impl Sync for LogCallback {}

fn check_status_and_close(status: i32, tsfn_state: &mut ThreadsafeFunctionState) -> napi::Result<()> {
    match status {
        Status::napi_ok => Ok(()),
        Status::napi_closing => {
            *tsfn_state = ThreadsafeFunctionState::Closing;
            Err(closing_error())
        }

        _ => Err(napi::Error::from_status(napi::Status::from(status))),
    }
}

fn closing_error() -> napi::Error {
    napi::Error::new(napi::Status::Closing, "callback is closing".into())
}

unsafe extern "C" fn finalize_callback(
    _raw_env: napi::sys::napi_env,
    finalize_data: *mut c_void,
    _finalize_hint: *mut c_void,
) {
    let state: Arc<Mutex<ThreadsafeFunctionState>> = Arc::from_raw(finalize_data.cast());
    let mut state = state.lock().unwrap();
    *state = ThreadsafeFunctionState::Closing;
}

unsafe extern "C" fn call_js(raw_env: napi_env, js_callback: napi_value, _context: *mut c_void, data: *mut c_void) {
    let value: Box<String> = Box::from_raw(data.cast());
    let env = Env::from_raw(raw_env);
    let _ = JsFunction::from_raw(raw_env, js_callback).map(|func| {
        let _ = env.create_string(&value).map(|value| func.call(None, &[value]));
    });
}
