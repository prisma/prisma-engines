use serde::Deserialize;

/// The implementation of the CLI getDmmf() utility and its JSON format.
pub mod internal {
    pub use dmmf::*;
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GetDmmfParams {
    prisma_schema: String,
}

pub(crate) fn get_dmmf(params: &str) -> String {
    let params: GetDmmfParams = match serde_json::from_str(params) {
        Ok(params) => params,
        Err(serde_err) => {
            panic!("Failed to deserialize GetDmmfParams: {}", serde_err,);
        }
    };

    get_dmmf_impl(params)
}

fn get_dmmf_impl(params: GetDmmfParams) -> String {
    // Note:
    // - if the Prisma schema is not valid, this panics
    internal::dmmf_json_from_schema(&params.prisma_schema)
}
