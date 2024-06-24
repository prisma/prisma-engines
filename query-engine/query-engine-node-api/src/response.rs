use napi::{bindgen_prelude::ToNapiValue, Env, NapiRaw};
use request_handlers::PrismaResponse;

pub enum NapiResponse {
    Json(String),
    Js(PrismaResponse),
}

impl ToNapiValue for NapiResponse {
    unsafe fn to_napi_value(env: napi::sys::napi_env, val: Self) -> napi::Result<napi::sys::napi_value> {
        let env = Env::from_raw(env);

        match val {
            NapiResponse::Json(json) => {
                let val = env.create_string(&json)?;

                Ok(val.raw())
            }
            NapiResponse::Js(mut response) => {
                let format = match response.is_sql_raw_response() {
                    true => request_handlers::ResponseFormat::Js,
                    false => request_handlers::ResponseFormat::Json,
                };

                response.set_format(format);

                let val = env.to_js_value(&response)?.coerce_to_object()?;

                Ok(val.raw())
            }
        }
    }
}
