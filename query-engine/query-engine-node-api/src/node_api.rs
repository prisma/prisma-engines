use napi::{Env, JsObject, Property};
use napi_derive::module_exports;

mod engine;
mod functions;

#[module_exports]
pub fn init(mut exports: JsObject, env: Env) -> napi::Result<()> {
    let query_engine = env.define_class(
        "QueryEngine",
        engine::constructor,
        &[
            Property::new(&env, "connect")?.with_method(engine::connect),
            Property::new(&env, "disconnect")?.with_method(engine::disconnect),
            Property::new(&env, "query")?.with_method(engine::query),
            Property::new(&env, "sdlSchema")?.with_method(engine::sdl_schema),
            Property::new(&env, "startTransaction")?.with_method(engine::start_transaction),
            Property::new(&env, "commitTransaction")?.with_method(engine::commit_transaction),
            Property::new(&env, "rollbackTransaction")?.with_method(engine::rollback_transaction),
        ],
    )?;

    exports.set_named_property("QueryEngine", query_engine)?;
    exports.create_named_method("version", functions::version)?;
    exports.create_named_method("getConfig", functions::get_config)?;
    exports.create_named_method("dmmf", functions::dmmf)?;
    exports.create_named_method("debugPanic", functions::debug_panic)?;

    Ok(())
}
