#[test]
fn code_actions_should_not_crash_on_validation_errors_with_mongodb() {
    let schema = r#"
        datasource db {
          provider = "mongodb"
          url      = env("DATABASE_URL")
        }

        generator client {
          provider = "prisma-client-js"
        }

        model A {
          1bar Bar[]
        }
    "#;

    let params = lsp_types::CodeActionParams {
        text_document: lsp_types::TextDocumentIdentifier {
            uri: "file:/path/to/schema.prisma".parse().unwrap(),
        },
        range: lsp_types::Range::default(),
        context: lsp_types::CodeActionContext::default(),
        work_done_progress_params: lsp_types::WorkDoneProgressParams { work_done_token: None },
        partial_result_params: lsp_types::PartialResultParams {
            partial_result_token: None,
        },
    };

    prisma_fmt::code_actions(
        serde_json::to_string_pretty(&[("schema.prisma", schema.to_owned())]).unwrap(),
        &serde_json::to_string_pretty(&params).unwrap(),
    );
}
