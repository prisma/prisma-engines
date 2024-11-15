use query_engine_tests::*;

/// * QUERY_BATCH_SIZE for testing is 10, configured in direnv.
/// * It should be called QUERY_CHUNK_SIZE instead, because it's a knob to configure query chunking
///  which is splitting queries with more arguments than accepted by the database, in multiple
///  queries.
/// * WASM versions of the engine don't allow for runtime configuration of this value so they default
///  the mininum supported by any database on a SQL family (eg. Postgres, MySQL, SQLite, SQL Server,
///  etc.) As such, in order to guarantee chunking happens, a large number of arguments --larger
///  than the default-- needs to be used, to have actual coverage of chunking code while exercising
///  WASM query engines.
#[test_suite(schema(schema))]
mod chunking {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query};

    #[test_suite(schema(schema))]
    mod reproductions {
        fn schema() -> String {
            let schema = indoc! {
              r#"
                model User {
                  #id(id, Int, @id)
                  posts Post[]
                }

                model Post {
                  #id(id, Int, @id)
                  user   User   @relation(fields: [userId], references: [id])
                  userId Int
                }
              "#
            };

            schema.to_owned()
        }

        async fn create_test_data(runner: &Runner) -> TestResult<()> {
            let n_users = 200;

            for i in 1..=n_users {
                let post_a_id = i * 2 - 1;
                let post_b_id = i * 2;

                create_user(
                    runner,
                    &format!(
                        r#"
                        {{ id: {}, posts: {{ create: [{{ id: {} }}, {{ id: {} }}] }} }}
                        "#,
                        i, post_a_id, post_b_id
                    ),
                )
                .await?;
            }

            Ok(())
        }

        async fn create_user(runner: &Runner, data: &str) -> TestResult<()> {
            runner
                .query(format!("mutation {{ createOneUser(data: {data}) {{ id }} }}"))
                .await?
                .assert_success();

            Ok(())
        }

        #[connector_test(exclude_features("relationJoins"))]
        // It used to error on D1 with
        // Error in performIO: Error: D1_ERROR: too many SQL variables at offset 395
        // see https://github.com/prisma/prisma/issues/23743
        async fn issue_23743(runner: Runner) -> TestResult<()> {
            create_test_data(&runner).await?;

            insta::assert_snapshot!(
              run_query!(&runner, r#"{
                findManyUser {
                  id, posts { id }
                }
              }
              "#),
              @r###"{"data":{"findManyUser":[{"id":1,"posts":[{"id":1},{"id":2}]},{"id":2,"posts":[{"id":3},{"id":4}]},{"id":3,"posts":[{"id":5},{"id":6}]},{"id":4,"posts":[{"id":7},{"id":8}]},{"id":5,"posts":[{"id":9},{"id":10}]},{"id":6,"posts":[{"id":11},{"id":12}]},{"id":7,"posts":[{"id":13},{"id":14}]},{"id":8,"posts":[{"id":15},{"id":16}]},{"id":9,"posts":[{"id":17},{"id":18}]},{"id":10,"posts":[{"id":19},{"id":20}]},{"id":11,"posts":[{"id":21},{"id":22}]},{"id":12,"posts":[{"id":23},{"id":24}]},{"id":13,"posts":[{"id":25},{"id":26}]},{"id":14,"posts":[{"id":27},{"id":28}]},{"id":15,"posts":[{"id":29},{"id":30}]},{"id":16,"posts":[{"id":31},{"id":32}]},{"id":17,"posts":[{"id":33},{"id":34}]},{"id":18,"posts":[{"id":35},{"id":36}]},{"id":19,"posts":[{"id":37},{"id":38}]},{"id":20,"posts":[{"id":39},{"id":40}]},{"id":21,"posts":[{"id":41},{"id":42}]},{"id":22,"posts":[{"id":43},{"id":44}]},{"id":23,"posts":[{"id":45},{"id":46}]},{"id":24,"posts":[{"id":47},{"id":48}]},{"id":25,"posts":[{"id":49},{"id":50}]},{"id":26,"posts":[{"id":51},{"id":52}]},{"id":27,"posts":[{"id":53},{"id":54}]},{"id":28,"posts":[{"id":55},{"id":56}]},{"id":29,"posts":[{"id":57},{"id":58}]},{"id":30,"posts":[{"id":59},{"id":60}]},{"id":31,"posts":[{"id":61},{"id":62}]},{"id":32,"posts":[{"id":63},{"id":64}]},{"id":33,"posts":[{"id":65},{"id":66}]},{"id":34,"posts":[{"id":67},{"id":68}]},{"id":35,"posts":[{"id":69},{"id":70}]},{"id":36,"posts":[{"id":71},{"id":72}]},{"id":37,"posts":[{"id":73},{"id":74}]},{"id":38,"posts":[{"id":75},{"id":76}]},{"id":39,"posts":[{"id":77},{"id":78}]},{"id":40,"posts":[{"id":79},{"id":80}]},{"id":41,"posts":[{"id":81},{"id":82}]},{"id":42,"posts":[{"id":83},{"id":84}]},{"id":43,"posts":[{"id":85},{"id":86}]},{"id":44,"posts":[{"id":87},{"id":88}]},{"id":45,"posts":[{"id":89},{"id":90}]},{"id":46,"posts":[{"id":91},{"id":92}]},{"id":47,"posts":[{"id":93},{"id":94}]},{"id":48,"posts":[{"id":95},{"id":96}]},{"id":49,"posts":[{"id":97},{"id":98}]},{"id":50,"posts":[{"id":99},{"id":100}]},{"id":51,"posts":[{"id":101},{"id":102}]},{"id":52,"posts":[{"id":103},{"id":104}]},{"id":53,"posts":[{"id":105},{"id":106}]},{"id":54,"posts":[{"id":107},{"id":108}]},{"id":55,"posts":[{"id":109},{"id":110}]},{"id":56,"posts":[{"id":111},{"id":112}]},{"id":57,"posts":[{"id":113},{"id":114}]},{"id":58,"posts":[{"id":115},{"id":116}]},{"id":59,"posts":[{"id":117},{"id":118}]},{"id":60,"posts":[{"id":119},{"id":120}]},{"id":61,"posts":[{"id":121},{"id":122}]},{"id":62,"posts":[{"id":123},{"id":124}]},{"id":63,"posts":[{"id":125},{"id":126}]},{"id":64,"posts":[{"id":127},{"id":128}]},{"id":65,"posts":[{"id":129},{"id":130}]},{"id":66,"posts":[{"id":131},{"id":132}]},{"id":67,"posts":[{"id":133},{"id":134}]},{"id":68,"posts":[{"id":135},{"id":136}]},{"id":69,"posts":[{"id":137},{"id":138}]},{"id":70,"posts":[{"id":139},{"id":140}]},{"id":71,"posts":[{"id":141},{"id":142}]},{"id":72,"posts":[{"id":143},{"id":144}]},{"id":73,"posts":[{"id":145},{"id":146}]},{"id":74,"posts":[{"id":147},{"id":148}]},{"id":75,"posts":[{"id":149},{"id":150}]},{"id":76,"posts":[{"id":151},{"id":152}]},{"id":77,"posts":[{"id":153},{"id":154}]},{"id":78,"posts":[{"id":155},{"id":156}]},{"id":79,"posts":[{"id":157},{"id":158}]},{"id":80,"posts":[{"id":159},{"id":160}]},{"id":81,"posts":[{"id":161},{"id":162}]},{"id":82,"posts":[{"id":163},{"id":164}]},{"id":83,"posts":[{"id":165},{"id":166}]},{"id":84,"posts":[{"id":167},{"id":168}]},{"id":85,"posts":[{"id":169},{"id":170}]},{"id":86,"posts":[{"id":171},{"id":172}]},{"id":87,"posts":[{"id":173},{"id":174}]},{"id":88,"posts":[{"id":175},{"id":176}]},{"id":89,"posts":[{"id":177},{"id":178}]},{"id":90,"posts":[{"id":179},{"id":180}]},{"id":91,"posts":[{"id":181},{"id":182}]},{"id":92,"posts":[{"id":183},{"id":184}]},{"id":93,"posts":[{"id":185},{"id":186}]},{"id":94,"posts":[{"id":187},{"id":188}]},{"id":95,"posts":[{"id":189},{"id":190}]},{"id":96,"posts":[{"id":191},{"id":192}]},{"id":97,"posts":[{"id":193},{"id":194}]},{"id":98,"posts":[{"id":195},{"id":196}]},{"id":99,"posts":[{"id":197},{"id":198}]},{"id":100,"posts":[{"id":199},{"id":200}]},{"id":101,"posts":[{"id":201},{"id":202}]},{"id":102,"posts":[{"id":203},{"id":204}]},{"id":103,"posts":[{"id":205},{"id":206}]},{"id":104,"posts":[{"id":207},{"id":208}]},{"id":105,"posts":[{"id":209},{"id":210}]},{"id":106,"posts":[{"id":211},{"id":212}]},{"id":107,"posts":[{"id":213},{"id":214}]},{"id":108,"posts":[{"id":215},{"id":216}]},{"id":109,"posts":[{"id":217},{"id":218}]},{"id":110,"posts":[{"id":219},{"id":220}]},{"id":111,"posts":[{"id":221},{"id":222}]},{"id":112,"posts":[{"id":223},{"id":224}]},{"id":113,"posts":[{"id":225},{"id":226}]},{"id":114,"posts":[{"id":227},{"id":228}]},{"id":115,"posts":[{"id":229},{"id":230}]},{"id":116,"posts":[{"id":231},{"id":232}]},{"id":117,"posts":[{"id":233},{"id":234}]},{"id":118,"posts":[{"id":235},{"id":236}]},{"id":119,"posts":[{"id":237},{"id":238}]},{"id":120,"posts":[{"id":239},{"id":240}]},{"id":121,"posts":[{"id":241},{"id":242}]},{"id":122,"posts":[{"id":243},{"id":244}]},{"id":123,"posts":[{"id":245},{"id":246}]},{"id":124,"posts":[{"id":247},{"id":248}]},{"id":125,"posts":[{"id":249},{"id":250}]},{"id":126,"posts":[{"id":251},{"id":252}]},{"id":127,"posts":[{"id":253},{"id":254}]},{"id":128,"posts":[{"id":255},{"id":256}]},{"id":129,"posts":[{"id":257},{"id":258}]},{"id":130,"posts":[{"id":259},{"id":260}]},{"id":131,"posts":[{"id":261},{"id":262}]},{"id":132,"posts":[{"id":263},{"id":264}]},{"id":133,"posts":[{"id":265},{"id":266}]},{"id":134,"posts":[{"id":267},{"id":268}]},{"id":135,"posts":[{"id":269},{"id":270}]},{"id":136,"posts":[{"id":271},{"id":272}]},{"id":137,"posts":[{"id":273},{"id":274}]},{"id":138,"posts":[{"id":275},{"id":276}]},{"id":139,"posts":[{"id":277},{"id":278}]},{"id":140,"posts":[{"id":279},{"id":280}]},{"id":141,"posts":[{"id":281},{"id":282}]},{"id":142,"posts":[{"id":283},{"id":284}]},{"id":143,"posts":[{"id":285},{"id":286}]},{"id":144,"posts":[{"id":287},{"id":288}]},{"id":145,"posts":[{"id":289},{"id":290}]},{"id":146,"posts":[{"id":291},{"id":292}]},{"id":147,"posts":[{"id":293},{"id":294}]},{"id":148,"posts":[{"id":295},{"id":296}]},{"id":149,"posts":[{"id":297},{"id":298}]},{"id":150,"posts":[{"id":299},{"id":300}]},{"id":151,"posts":[{"id":301},{"id":302}]},{"id":152,"posts":[{"id":303},{"id":304}]},{"id":153,"posts":[{"id":305},{"id":306}]},{"id":154,"posts":[{"id":307},{"id":308}]},{"id":155,"posts":[{"id":309},{"id":310}]},{"id":156,"posts":[{"id":311},{"id":312}]},{"id":157,"posts":[{"id":313},{"id":314}]},{"id":158,"posts":[{"id":315},{"id":316}]},{"id":159,"posts":[{"id":317},{"id":318}]},{"id":160,"posts":[{"id":319},{"id":320}]},{"id":161,"posts":[{"id":321},{"id":322}]},{"id":162,"posts":[{"id":323},{"id":324}]},{"id":163,"posts":[{"id":325},{"id":326}]},{"id":164,"posts":[{"id":327},{"id":328}]},{"id":165,"posts":[{"id":329},{"id":330}]},{"id":166,"posts":[{"id":331},{"id":332}]},{"id":167,"posts":[{"id":333},{"id":334}]},{"id":168,"posts":[{"id":335},{"id":336}]},{"id":169,"posts":[{"id":337},{"id":338}]},{"id":170,"posts":[{"id":339},{"id":340}]},{"id":171,"posts":[{"id":341},{"id":342}]},{"id":172,"posts":[{"id":343},{"id":344}]},{"id":173,"posts":[{"id":345},{"id":346}]},{"id":174,"posts":[{"id":347},{"id":348}]},{"id":175,"posts":[{"id":349},{"id":350}]},{"id":176,"posts":[{"id":351},{"id":352}]},{"id":177,"posts":[{"id":353},{"id":354}]},{"id":178,"posts":[{"id":355},{"id":356}]},{"id":179,"posts":[{"id":357},{"id":358}]},{"id":180,"posts":[{"id":359},{"id":360}]},{"id":181,"posts":[{"id":361},{"id":362}]},{"id":182,"posts":[{"id":363},{"id":364}]},{"id":183,"posts":[{"id":365},{"id":366}]},{"id":184,"posts":[{"id":367},{"id":368}]},{"id":185,"posts":[{"id":369},{"id":370}]},{"id":186,"posts":[{"id":371},{"id":372}]},{"id":187,"posts":[{"id":373},{"id":374}]},{"id":188,"posts":[{"id":375},{"id":376}]},{"id":189,"posts":[{"id":377},{"id":378}]},{"id":190,"posts":[{"id":379},{"id":380}]},{"id":191,"posts":[{"id":381},{"id":382}]},{"id":192,"posts":[{"id":383},{"id":384}]},{"id":193,"posts":[{"id":385},{"id":386}]},{"id":194,"posts":[{"id":387},{"id":388}]},{"id":195,"posts":[{"id":389},{"id":390}]},{"id":196,"posts":[{"id":391},{"id":392}]},{"id":197,"posts":[{"id":393},{"id":394}]},{"id":198,"posts":[{"id":395},{"id":396}]},{"id":199,"posts":[{"id":397},{"id":398}]},{"id":200,"posts":[{"id":399},{"id":400}]}]}}"###
            );

            Ok(())
        }

        #[connector_test(exclude_features("relationJoins"), exclude(Sqlite("cfd1")))]
        // It errors on D1 with
        // Error in performIO: Error: D1_ERROR: Expression tree is too large (maximum depth 100)
        // see https://github.com/prisma/prisma/issues/23919
        async fn issue_23919(runner: Runner) -> TestResult<()> {
            create_test_data(&runner).await?;

            let posts_as_str = run_query!(
                &runner,
                r#"{
                  findManyPost {
                    id
                  }
                }
                "#
            );
            let posts_as_json = serde_json::from_str::<serde_json::Value>(&posts_as_str).unwrap();
            let ids_vec = posts_as_json.as_object().unwrap()["data"].as_object().unwrap()["findManyPost"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x["id"].as_i64().unwrap())
                .collect::<Vec<i64>>();

            let posts_as_graphql: Vec<String> = ids_vec.into_iter().map(|id| format!("{{ id: {} }}", id)).collect();
            assert_eq!(posts_as_graphql.len(), 400);

            let query = format!("{{ id: 201, posts: {{ connect: [{}] }} }}", posts_as_graphql.join(", "));

            create_user(&runner, &query).await?;

            Ok(())
        }
    }

    fn schema() -> String {
        let schema = indoc! {
            r#"
              model A {
                #id(id, Int, @id)
                b_id Int
                c_id Int
                text String

                b B @relation(fields: [b_id], references: [id])
                c C @relation(fields: [c_id], references: [id])
              }

              model B {
                #id(id, Int, @id)
                as A[]
              }

              model C {
                #id(id, Int, @id)
                as A[]
              }
            "#
        };

        schema.to_owned()
    }

    // "chunking of IN queries" should "work when having more than the specified amount of items"
    // TODO(joins): Excluded because we have no support for chunked queries with joins. In practice, it should happen under much less circumstances
    // TODO(joins): than with the query-based strategy, because we don't issue `WHERE IN (parent_ids)` queries anymore to resolve relations.
    #[connector_test(exclude_features("relationJoins"))]
    async fn in_more_items(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManyA(where: { id: { in: [5,4,3,2,1,1,1,2,3,4,5,6,7,6,5,4,3,2,1,2,3,4,5,6] }}) { id }
            }"# }),
          @r###"{"data":{"findManyA":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5}]}}"###
        );

        Ok(())
    }

    // "ascending ordering of chunked IN queries" should "work when having more than the specified amount of items"
    // TODO(joins): Excluded because we have no support for chunked queries with joins. In practice, it should happen under much less circumstances
    // TODO(joins): than with the query-based strategy, because we don't issue `WHERE IN (parent_ids)` queries anymore to resolve relations.
    #[connector_test(exclude_features("relationJoins"))]
    async fn asc_in_ordering(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManyA(where: { id: { in: [5,4,3,2,1,2,1,1,3,4,5,6,7,6,5,4,3,2,1,2,3,4,5,6] }}, orderBy: { id: asc }) { id }
            }"# }),
          @r###"{"data":{"findManyA":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5}]}}"###
        );

        Ok(())
    }

    // "ascending ordering of chunked IN queries" should "work when having more than the specified amount of items"
    // TODO(joins): Excluded because we have no support for chunked queries with joins. In practice, it should happen under much less circumstances
    // TODO(joins): than with the query-based strategy, because we don't issue `WHERE IN (parent_ids)` queries anymore to resolve relations.
    #[connector_test(exclude_features("relationJoins"))]
    async fn desc_in_ordering(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManyA(where: {id: { in: [5,4,3,2,1,1,1,2,3,4,5,6,7,6,5,4,3,2,1,2,3,4,5,6] }}, orderBy: { id: desc }) { id }
            }"# }),
          @r###"{"data":{"findManyA":[{"id":5},{"id":4},{"id":3},{"id":2},{"id":1}]}}"###
        );

        Ok(())
    }

    #[connector_test(exclude(MongoDb, Sqlite("cfd1")))]
    async fn order_by_aggregation_should_fail(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        assert_error!(
            runner,
            with_id_excess!(&runner, "query { findManyA(where: {id: { in: [:id_list:] }}, orderBy: { b: { as: { _count: asc } } } ) { id } }"),
            2029 // QueryParameterLimitExceeded
        );

        Ok(())
    }

    #[connector_test(capabilities(FullTextSearchWithoutIndex), exclude(MongoDb))]
    async fn order_by_relevance_should_fail(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        assert_error!(
            runner,
            with_id_excess!(
                &runner,
                r#"query { findManyA(where: {id: { in: [:id_list:] }}, orderBy: { _relevance: { fields: text, search: "something", sort: asc } } ) { id } }"#
            ),
            2029 // QueryParameterLimitExceeded
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_a(
            runner,
            r#"{ id: 1, text: "", b: { create: { id: 1 }} c: { create: { id: 1 }} }"#,
        )
        .await?;
        create_a(
            runner,
            r#"{ id: 2, text: "", b: { connect: { id: 1 }} c: { create: { id: 2 }} }"#,
        )
        .await?;
        create_a(
            runner,
            r#"{ id: 3, text: "", b: { create: { id: 3 }} c: { create: { id: 3 }} }"#,
        )
        .await?;
        create_a(
            runner,
            r#"{ id: 4, text: "", b: { create: { id: 4 }} c: { create: { id: 4 }} }"#,
        )
        .await?;
        create_a(
            runner,
            r#"{ id: 5, text: "", b: { create: { id: 5 }} c: { create: { id: 5 }} }"#,
        )
        .await?;

        Ok(())
    }

    async fn create_a(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneA(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();

        Ok(())
    }
}
