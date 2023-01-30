use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GetDmmfParams {
    prisma_schema: String,
}

pub(crate) fn get_dmmf(params: &str) -> String {
    let params: GetDmmfParams = match serde_json::from_str(params) {
        Ok(params) => params,
        Err(serde_err) => {
            panic!("Failed to deserialize GetDmmfParams: {serde_err}");
        }
    };

    // if the Prisma schema is not valid, this panics
    dmmf::dmmf_json_from_schema(&params.prisma_schema)
}
