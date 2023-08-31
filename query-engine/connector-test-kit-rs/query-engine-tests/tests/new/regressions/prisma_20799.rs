use query_engine_tests::*;

// before the fix to https://github.com/prisma/prisma/issues/20799, this test would consistently
// run for multiple minutes and crash with an OOM error on a fast desktop machine with 32GB of RAM.
#[test_suite(schema(schema), only(Sqlite))]
mod regression {
    fn schema() -> String {
        indoc!(
            r#"
model Connection {
    id                                   Int    @id @default(autoincrement())
    uid                                  String @unique
    ownerId                              Int
    atlassianOnPremiseOAuthCredentialsId Int?   @unique
    bitbucketCloudOAuthCredentialsId     Int?   @unique
    genericAppCredentialsId              Int?   @unique
    gitlabOAuthCredentialsId             Int?   @unique
    googleSheetsOAuthCredentialsId       Int?   @unique
    githubOAuthCredentialsId             Int?   @unique
    mondayOAuthCredentialsId             Int?   @unique
    serviceNowOAuthCredentialsId         Int?   @unique
    bitbucketOnPremiseOAuthCredentialsId Int?   @unique
    salesforceOAuthCredentialsId         Int?   @unique
    tempoCloudOAuthCredentialsId         Int?   @unique
    slackCredentialsId                   Int?
    jsmCloudAssetsApiKeyCredentialsId    Int?   @unique
    googleCalendarOAuthCredentialsId     Int?   @unique
    microsoftOAuthCredentialsId          Int?   @unique
    zoomOAuthCredentialsId               Int?   @unique
    statuspageApiKeyCredentialsId        Int?   @unique
    trelloApiKeyCredentialsId            Int?   @unique
    opsgenieApiKeyCredentialsId          Int?   @unique
    one                                  Int?   @unique
    two                                  Int?   @unique
    three                                Int?   @unique
    four                                 Int?   @unique
    five                                 Int?   @unique
    six                                  Int?   @unique
    seven                                Int?   @unique
    eight                                Int?   @unique
    nine                                 Int?   @unique
    ten                                  Int?   @unique
}
        "#
        )
        .to_owned()
    }

    #[connector_test]
    async fn repro(runner: Runner) -> TestResult<()> {
        let query = indoc!(
            r#"
query {
    findManyConnection(
        where: {
            ownerId: 100,
        },
    ) { id }
}
        "#
        );
        runner.query(query).await?.assert_success();
        Ok(())
    }
}
