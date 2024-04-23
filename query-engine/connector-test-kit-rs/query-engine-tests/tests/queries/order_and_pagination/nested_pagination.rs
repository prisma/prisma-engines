use query_engine_tests::*;

#[test_suite(schema(schema))]
mod nested_pagination {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model Top {
              #id(id, Int, @id)
              t      String   @unique

              middles Middle[]
            }

            model Middle {
              #id(id, Int, @id)
              m       String   @unique
              top_id Int

              top     Top      @relation(fields: [top_id], references: [id])
              bottoms Bottom[]
            }

            model Bottom {
              #id(id, Int, @id)
              b         String @unique
              middle_id Int

              middle Middle @relation(fields: [middle_id], references: [id])
            }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn all_data_there(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop{t, middles{ m, bottoms {b}}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[{"m":"M11","bottoms":[{"b":"B111"},{"b":"B112"},{"b":"B113"}]},{"m":"M12","bottoms":[{"b":"B121"},{"b":"B122"},{"b":"B123"}]},{"m":"M13","bottoms":[{"b":"B131"},{"b":"B132"},{"b":"B133"}]}]},{"t":"T2","middles":[{"m":"M21","bottoms":[{"b":"B211"},{"b":"B212"},{"b":"B213"}]},{"m":"M22","bottoms":[{"b":"B221"},{"b":"B222"},{"b":"B223"}]},{"m":"M23","bottoms":[{"b":"B231"},{"b":"B232"},{"b":"B233"}]}]},{"t":"T3","middles":[{"m":"M31","bottoms":[{"b":"B311"},{"b":"B312"},{"b":"B313"}]},{"m":"M32","bottoms":[{"b":"B321"},{"b":"B322"},{"b":"B323"}]},{"m":"M33","bottoms":[{"b":"B331"},{"b":"B332"},{"b":"B333"}]}]}]}}"###
        );

        Ok(())
    }

    /******************
     * Cursor tests. *
     *****************/

    // should return all items after and including the cursor and return nothing for other tops
    #[connector_test]
    async fn mid_lvl_cursor(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop{t, middles(cursor: { m: "M22" }, orderBy: { id: asc }){ m }}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[]},{"t":"T2","middles":[{"m":"M22"},{"m":"M23"}]},{"t":"T3","middles":[]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop(skip: 1, take: 1){t, middles(cursor: { m: "M22" }, orderBy: { id: asc }){ m }}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T2","middles":[{"m":"M22"},{"m":"M23"}]}]}}"###
        );

        Ok(())
    }

    /****************
     * Skip tests. *
     ***************/

    // should skip the first item
    #[connector_test]
    async fn mid_lvl_skip_1(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop{t, middles(skip: 1){m}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[{"m":"M12"},{"m":"M13"}]},{"t":"T2","middles":[{"m":"M22"},{"m":"M23"}]},{"t":"T3","middles":[{"m":"M32"},{"m":"M33"}]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop(take: 1){t, middles(skip: 1){m}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[{"m":"M12"},{"m":"M13"}]}]}}"###
        );

        Ok(())
    }

    // should "skip all items"
    #[connector_test]
    async fn mid_lvl_skip_3(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop{t, middles(skip: 3){m}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[]},{"t":"T2","middles":[]},{"t":"T3","middles":[]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop(take: 1){t, middles(skip: 3){m}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[]}]}}"###
        );

        Ok(())
    }

    // should "skip all items"
    #[connector_test]
    async fn mid_lvl_skip_4(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop{t, middles(skip: 4){m}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[]},{"t":"T2","middles":[]},{"t":"T3","middles":[]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop(take: 1){t, middles(skip: 4){m}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[]}]}}"###
        );

        Ok(())
    }

    // should "skip no items"
    #[connector_test]
    async fn bottom_lvl_skip_0(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop{middles{bottoms(skip: 0){b}}}
          }"#),
          @r###"{"data":{"findManyTop":[{"middles":[{"bottoms":[{"b":"B111"},{"b":"B112"},{"b":"B113"}]},{"bottoms":[{"b":"B121"},{"b":"B122"},{"b":"B123"}]},{"bottoms":[{"b":"B131"},{"b":"B132"},{"b":"B133"}]}]},{"middles":[{"bottoms":[{"b":"B211"},{"b":"B212"},{"b":"B213"}]},{"bottoms":[{"b":"B221"},{"b":"B222"},{"b":"B223"}]},{"bottoms":[{"b":"B231"},{"b":"B232"},{"b":"B233"}]}]},{"middles":[{"bottoms":[{"b":"B311"},{"b":"B312"},{"b":"B313"}]},{"bottoms":[{"b":"B321"},{"b":"B322"},{"b":"B323"}]},{"bottoms":[{"b":"B331"},{"b":"B332"},{"b":"B333"}]}]}]}}"###
        );

        Ok(())
    }

    // should "skip no items"
    #[connector_test]
    async fn bottom_lvl_skip_1(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop{middles{bottoms(skip: 1){b}}}
          }"#),
          @r###"{"data":{"findManyTop":[{"middles":[{"bottoms":[{"b":"B112"},{"b":"B113"}]},{"bottoms":[{"b":"B122"},{"b":"B123"}]},{"bottoms":[{"b":"B132"},{"b":"B133"}]}]},{"middles":[{"bottoms":[{"b":"B212"},{"b":"B213"}]},{"bottoms":[{"b":"B222"},{"b":"B223"}]},{"bottoms":[{"b":"B232"},{"b":"B233"}]}]},{"middles":[{"bottoms":[{"b":"B312"},{"b":"B313"}]},{"bottoms":[{"b":"B322"},{"b":"B323"}]},{"bottoms":[{"b":"B332"},{"b":"B333"}]}]}]}}"###
        );

        Ok(())
    }

    // should "skip no items"
    #[connector_test]
    async fn bottom_lvl_skip_3(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
                findManyTop{middles{bottoms(skip: 3){b}}}
              }"#),
          @r###"{"data":{"findManyTop":[{"middles":[{"bottoms":[]},{"bottoms":[]},{"bottoms":[]}]},{"middles":[{"bottoms":[]},{"bottoms":[]},{"bottoms":[]}]},{"middles":[{"bottoms":[]},{"bottoms":[]},{"bottoms":[]}]}]}}"###
        );

        Ok(())
    }

    // should "skip no items"
    #[connector_test]
    async fn bottom_lvl_skip_4(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
                    findManyTop{middles{bottoms(skip: 4){b}}}
                  }"#),
          @r###"{"data":{"findManyTop":[{"middles":[{"bottoms":[]},{"bottoms":[]},{"bottoms":[]}]},{"middles":[{"bottoms":[]},{"bottoms":[]},{"bottoms":[]}]},{"middles":[{"bottoms":[]},{"bottoms":[]},{"bottoms":[]}]}]}}"###
        );

        Ok(())
    }

    /**************
     * Take tests *
     **************/

    // should return no items
    #[connector_test]
    async fn mid_lvl_take_0(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop{t, middles(take: 0){m}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[]},{"t":"T2","middles":[]},{"t":"T3","middles":[]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop(take: 1){t, middles(take: 0){m}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[]}]}}"###
        );

        Ok(())
    }

    // should "return the first item"
    #[connector_test]
    async fn mid_lvl_take_1(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop{t, middles(take: 1){m}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[{"m":"M11"}]},{"t":"T2","middles":[{"m":"M21"}]},{"t":"T3","middles":[{"m":"M31"}]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop(take: 1){t, middles(take: 1){m}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[{"m":"M11"}]}]}}"###
        );

        Ok(())
    }

    // should "return all items"
    #[connector_test]
    async fn mid_lvl_take_3(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
                findManyTop{t, middles(take: 3){m}}
              }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[{"m":"M11"},{"m":"M12"},{"m":"M13"}]},{"t":"T2","middles":[{"m":"M21"},{"m":"M22"},{"m":"M23"}]},{"t":"T3","middles":[{"m":"M31"},{"m":"M32"},{"m":"M33"}]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
                findManyTop(take: 1){t, middles(take: 3){m}}
              }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[{"m":"M11"},{"m":"M12"},{"m":"M13"}]}]}}"###
        );

        Ok(())
    }

    // should "return all items"
    #[connector_test]
    async fn mid_lvl_take_4(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
                    findManyTop{t, middles(take: 4){m}}
                  }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[{"m":"M11"},{"m":"M12"},{"m":"M13"}]},{"t":"T2","middles":[{"m":"M21"},{"m":"M22"},{"m":"M23"}]},{"t":"T3","middles":[{"m":"M31"},{"m":"M32"},{"m":"M33"}]}]}}"###
        );

        Ok(())
    }

    // should "return no items"
    #[connector_test]
    async fn bottom_lvl_take_0(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop{middles{bottoms(take: 0){b}}}
          }"#),
          @r###"{"data":{"findManyTop":[{"middles":[{"bottoms":[]},{"bottoms":[]},{"bottoms":[]}]},{"middles":[{"bottoms":[]},{"bottoms":[]},{"bottoms":[]}]},{"middles":[{"bottoms":[]},{"bottoms":[]},{"bottoms":[]}]}]}}"###
        );

        Ok(())
    }

    // should "return the first item"
    #[connector_test]
    async fn bottom_lvl_take_1(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop{middles{bottoms(take:1){b}}}
          }"#),
          @r###"{"data":{"findManyTop":[{"middles":[{"bottoms":[{"b":"B111"}]},{"bottoms":[{"b":"B121"}]},{"bottoms":[{"b":"B131"}]}]},{"middles":[{"bottoms":[{"b":"B211"}]},{"bottoms":[{"b":"B221"}]},{"bottoms":[{"b":"B231"}]}]},{"middles":[{"bottoms":[{"b":"B311"}]},{"bottoms":[{"b":"B321"}]},{"bottoms":[{"b":"B331"}]}]}]}}"###
        );

        Ok(())
    }

    // should "return all items"
    #[connector_test]
    async fn bottom_lvl_take_3(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
                findManyTop{middles{bottoms(take:3){b}}}
              }"#),
          @r###"{"data":{"findManyTop":[{"middles":[{"bottoms":[{"b":"B111"},{"b":"B112"},{"b":"B113"}]},{"bottoms":[{"b":"B121"},{"b":"B122"},{"b":"B123"}]},{"bottoms":[{"b":"B131"},{"b":"B132"},{"b":"B133"}]}]},{"middles":[{"bottoms":[{"b":"B211"},{"b":"B212"},{"b":"B213"}]},{"bottoms":[{"b":"B221"},{"b":"B222"},{"b":"B223"}]},{"bottoms":[{"b":"B231"},{"b":"B232"},{"b":"B233"}]}]},{"middles":[{"bottoms":[{"b":"B311"},{"b":"B312"},{"b":"B313"}]},{"bottoms":[{"b":"B321"},{"b":"B322"},{"b":"B323"}]},{"bottoms":[{"b":"B331"},{"b":"B332"},{"b":"B333"}]}]}]}}"###
        );

        Ok(())
    }

    // should "return all items"
    #[connector_test]
    async fn bottom_lvl_take_4(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
                        findManyTop{middles{bottoms(take:4){b}}}
                      }"#),
          @r###"{"data":{"findManyTop":[{"middles":[{"bottoms":[{"b":"B111"},{"b":"B112"},{"b":"B113"}]},{"bottoms":[{"b":"B121"},{"b":"B122"},{"b":"B123"}]},{"bottoms":[{"b":"B131"},{"b":"B132"},{"b":"B133"}]}]},{"middles":[{"bottoms":[{"b":"B211"},{"b":"B212"},{"b":"B213"}]},{"bottoms":[{"b":"B221"},{"b":"B222"},{"b":"B223"}]},{"bottoms":[{"b":"B231"},{"b":"B232"},{"b":"B233"}]}]},{"middles":[{"bottoms":[{"b":"B311"},{"b":"B312"},{"b":"B313"}]},{"bottoms":[{"b":"B321"},{"b":"B322"},{"b":"B323"}]},{"bottoms":[{"b":"B331"},{"b":"B332"},{"b":"B333"}]}]}]}}"###
        );

        Ok(())
    }

    // should "return the last item"
    #[connector_test]
    async fn mid_lvl_take_minus_1(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop(orderBy: {t: asc}){t, middles(take: -1, orderBy: { id: asc }){m}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[{"m":"M13"}]},{"t":"T2","middles":[{"m":"M23"}]},{"t":"T3","middles":[{"m":"M33"}]}]}}"###
        );

        Ok(())
    }

    // should "return all items"
    #[connector_test]
    async fn mid_lvl_take_minus_3(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop(orderBy: {t: asc}){t, middles(take: -3, orderBy: { id: asc }) {m}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[{"m":"M11"},{"m":"M12"},{"m":"M13"}]},{"t":"T2","middles":[{"m":"M21"},{"m":"M22"},{"m":"M23"}]},{"t":"T3","middles":[{"m":"M31"},{"m":"M32"},{"m":"M33"}]}]}}"###
        );

        Ok(())
    }

    // should "return all items"
    #[connector_test]
    async fn mid_lvl_take_minus_4(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
                findManyTop(orderBy: {t: asc}){t, middles(take: -4, orderBy: { id: asc }) {m}}
              }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[{"m":"M11"},{"m":"M12"},{"m":"M13"}]},{"t":"T2","middles":[{"m":"M21"},{"m":"M22"},{"m":"M23"}]},{"t":"T3","middles":[{"m":"M31"},{"m":"M32"},{"m":"M33"}]}]}}"###
        );

        Ok(())
    }

    // should "return the last item"
    #[connector_test]
    async fn bottom_lvl_take_minus_1(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop{middles(orderBy: {m: asc}){bottoms(take: -1, orderBy: { id: asc }){b}}}
          }"#),
          @r###"{"data":{"findManyTop":[{"middles":[{"bottoms":[{"b":"B113"}]},{"bottoms":[{"b":"B123"}]},{"bottoms":[{"b":"B133"}]}]},{"middles":[{"bottoms":[{"b":"B213"}]},{"bottoms":[{"b":"B223"}]},{"bottoms":[{"b":"B233"}]}]},{"middles":[{"bottoms":[{"b":"B313"}]},{"bottoms":[{"b":"B323"}]},{"bottoms":[{"b":"B333"}]}]}]}}"###
        );

        Ok(())
    }

    // should "return all items"
    #[connector_test]
    async fn bottom_lvl_take_minus_3(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
                findManyTop{middles(orderBy: {m: asc}){bottoms(take: -3, orderBy: { id: asc }){b}}}
              }"#),
          @r###"{"data":{"findManyTop":[{"middles":[{"bottoms":[{"b":"B111"},{"b":"B112"},{"b":"B113"}]},{"bottoms":[{"b":"B121"},{"b":"B122"},{"b":"B123"}]},{"bottoms":[{"b":"B131"},{"b":"B132"},{"b":"B133"}]}]},{"middles":[{"bottoms":[{"b":"B211"},{"b":"B212"},{"b":"B213"}]},{"bottoms":[{"b":"B221"},{"b":"B222"},{"b":"B223"}]},{"bottoms":[{"b":"B231"},{"b":"B232"},{"b":"B233"}]}]},{"middles":[{"bottoms":[{"b":"B311"},{"b":"B312"},{"b":"B313"}]},{"bottoms":[{"b":"B321"},{"b":"B322"},{"b":"B323"}]},{"bottoms":[{"b":"B331"},{"b":"B332"},{"b":"B333"}]}]}]}}"###
        );

        Ok(())
    }

    // should "return all items"
    #[connector_test]
    async fn bottom_lvl_take_minus_4(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
                    findManyTop{middles(orderBy: {m: asc}){bottoms(take: -4, orderBy: { id: asc }){b}}}
                  }"#),
          @r###"{"data":{"findManyTop":[{"middles":[{"bottoms":[{"b":"B111"},{"b":"B112"},{"b":"B113"}]},{"bottoms":[{"b":"B121"},{"b":"B122"},{"b":"B123"}]},{"bottoms":[{"b":"B131"},{"b":"B132"},{"b":"B133"}]}]},{"middles":[{"bottoms":[{"b":"B211"},{"b":"B212"},{"b":"B213"}]},{"bottoms":[{"b":"B221"},{"b":"B222"},{"b":"B223"}]},{"bottoms":[{"b":"B231"},{"b":"B232"},{"b":"B233"}]}]},{"middles":[{"bottoms":[{"b":"B311"},{"b":"B312"},{"b":"B313"}]},{"bottoms":[{"b":"B321"},{"b":"B322"},{"b":"B323"}]},{"bottoms":[{"b":"B331"},{"b":"B332"},{"b":"B333"}]}]}]}}"###
        );

        Ok(())
    }

    /**********************
     * Skip + Take tests *
     *********************/

    // should "return the second item"
    #[connector_test]
    async fn top_lvl_skip_1_take_1(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop(skip: 1, take: 1){t, middles(orderBy: { m: asc }){m}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T2","middles":[{"m":"M21"},{"m":"M22"},{"m":"M23"}]}]}}"###
        );

        Ok(())
    }

    // should "return only the last two items"
    #[connector_test]
    async fn top_lvl_skip_1_take_3(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop(skip: 1, take: 3){t, middles(orderBy: { m: asc }){m}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T2","middles":[{"m":"M21"},{"m":"M22"},{"m":"M23"}]},{"t":"T3","middles":[{"m":"M31"},{"m":"M32"},{"m":"M33"}]}]}}"###
        );

        Ok(())
    }

    // should "return the second"
    #[connector_test]
    async fn mid_lvl_skip_1_take_1(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop{t, middles(skip: 1, take: 1){m}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[{"m":"M12"}]},{"t":"T2","middles":[{"m":"M22"}]},{"t":"T3","middles":[{"m":"M32"}]}]}}"###
        );

        Ok(())
    }

    // should "return the last two items"
    #[connector_test]
    async fn mid_lvl_skip_1_take_3(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop{t, middles(skip: 1, take: 3){m}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[{"m":"M12"},{"m":"M13"}]},{"t":"T2","middles":[{"m":"M22"},{"m":"M23"}]},{"t":"T3","middles":[{"m":"M32"},{"m":"M33"}]}]}}"###
        );

        Ok(())
    }

    // should "return only the first two items"
    #[connector_test]
    async fn top_lvl_skip_1_take_minus_3(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop(skip: 1, take: -3, orderBy: { id: asc }){t, middles(orderBy: { m: asc }){m}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[{"m":"M11"},{"m":"M12"},{"m":"M13"}]},{"t":"T2","middles":[{"m":"M21"},{"m":"M22"},{"m":"M23"}]}]}}"###
        );

        Ok(())
    }

    // should "return the second"
    #[connector_test]
    async fn mid_lvl_skip_1_take_minus_1(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop(orderBy: { t: asc }){t, middles(skip: 1, take: -1, orderBy: { id: asc }){m}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[{"m":"M12"}]},{"t":"T2","middles":[{"m":"M22"}]},{"t":"T3","middles":[{"m":"M32"}]}]}}"###
        );

        Ok(())
    }

    // should "return the first two items"
    #[connector_test]
    async fn mid_lvl_skip_1_take_minus_3(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop(orderBy: { t: asc }){t, middles(skip: 1, take: -3, orderBy: { id: asc }){m}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[{"m":"M11"},{"m":"M12"}]},{"t":"T2","middles":[{"m":"M21"},{"m":"M22"}]},{"t":"T3","middles":[{"m":"M31"},{"m":"M32"}]}]}}"###
        );

        Ok(())
    }

    /*************************
     * Skip + take + order. *
     ************************/

    // should "return the last item"
    #[connector_test]
    async fn mid_order_by_take_1(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop(orderBy: { t: asc }){t, middles(orderBy: { m: desc }, take: 1){m}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[{"m":"M13"}]},{"t":"T2","middles":[{"m":"M23"}]},{"t":"T3","middles":[{"m":"M33"}]}]}}"###
        );

        Ok(())
    }

    // should "return all items in reverse order"
    #[connector_test]
    async fn mid_lvl_order_by_take_3(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTop(orderBy: { t: asc }){t, middles(orderBy: { m: desc }, take: 3){m}}
          }"#),
          @r###"{"data":{"findManyTop":[{"t":"T1","middles":[{"m":"M13"},{"m":"M12"},{"m":"M11"}]},{"t":"T2","middles":[{"m":"M23"},{"m":"M22"},{"m":"M21"}]},{"t":"T3","middles":[{"m":"M33"},{"m":"M32"},{"m":"M31"}]}]}}"###
        );

        Ok(())
    }

    /***************
     * M:N tests. *
     **************/
    // Special case: m:n relations, child is connected to many parents, using cursor pagination
    // A1 <> B1, B2, B3
    // A2 <> B2
    // A3
    // "A many-to-many relationship with multiple connected children" should "return all items correctly with nested cursor pagination"
    #[connector_test(schema(simple_m2m))]
    async fn m2m_many_children_nested_cursor(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
              createOneModelA(
                data: {
                  id: "A1"
                  manyB: {
                    connectOrCreate: [
                      { where: { id: "B1" }, create: { id: "B1" } }
                      { where: { id: "B2" }, create: { id: "B2" } }
                      { where: { id: "B3" }, create: { id: "B3" } }
                    ]
                  }
                }
              ) {
                id
                manyB {
                  id
                }
              }
            }"#
          ),
          @r###"{"data":{"createOneModelA":{"id":"A1","manyB":[{"id":"B1"},{"id":"B2"},{"id":"B3"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"
            mutation {
              createOneModelA(
                data: {
                  id: "A2"
                  manyB: {
                    connectOrCreate: [
                      { where: { id: "B2" }, create: { id: "B2" } }
                    ]
                  }
                }
              ) {
                id
                manyB {
                  id
                }
              }
            }"#),
          @r###"{"data":{"createOneModelA":{"id":"A2","manyB":[{"id":"B2"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"
            mutation{
              createOneModelA(data: {
                id: "A3"
              }) {
                id
                manyB {
                  id
                }
              }
            }
          "#),
          @r###"{"data":{"createOneModelA":{"id":"A3","manyB":[]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"
            query {
              findManyModelA {
                id
                manyB(cursor: {
                  id: "B2"
                }) {
                  id
                }
              }
            }
          "#),
          @r###"{"data":{"findManyModelA":[{"id":"A1","manyB":[{"id":"B2"},{"id":"B3"}]},{"id":"A2","manyB":[{"id":"B2"}]},{"id":"A3","manyB":[]}]}}"###
        );

        Ok(())
    }

    // Special case: m:n relations, child is connected to many parents, using cursor pagination
    // A1 <> B1, B2, B3, B4, B5, B6
    // A2 <> B2, B3, B5, B7, B8
    // A3
    //"A many-to-many relationship with multiple connected children" should "return all items correctly with nested cursor pagination and skip / take"
    #[connector_test(schema(simple_m2m))]
    async fn m2m_many_children_nested_cursor_skip_take(runner: Runner) -> TestResult<()> {
        // >>> Begin create test data

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModelA(
              data: {
                id: "A1"
                manyB: {
                  connectOrCreate: [
                    { where: { id: "B1" }, create: { id: "B1" } }
                    { where: { id: "B2" }, create: { id: "B2" } }
                    { where: { id: "B3" }, create: { id: "B3" } }
                    { where: { id: "B4" }, create: { id: "B4" } }
                    { where: { id: "B5" }, create: { id: "B5" } }
                    { where: { id: "B6" }, create: { id: "B6" } }
                  ]
                }
              }
            ) {
              id
              manyB {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":"A1","manyB":[{"id":"B1"},{"id":"B2"},{"id":"B3"},{"id":"B4"},{"id":"B5"},{"id":"B6"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModelA(
              data: {
                id: "A2"
                manyB: {
                  connectOrCreate: [
                    { where: { id: "B2" }, create: { id: "B2" } },
                    { where: { id: "B3" }, create: { id: "B3" } }
                    { where: { id: "B5" }, create: { id: "B5" } }
                    { where: { id: "B7" }, create: { id: "B7" } }
                    { where: { id: "B8" }, create: { id: "B8" } }
                  ]
                }
              }
            ) {
              id
              manyB {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":"A2","manyB":[{"id":"B2"},{"id":"B3"},{"id":"B5"},{"id":"B7"},{"id":"B8"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation{
            createOneModelA(data: {
              id: "A3"
            }) {
              id
              manyB {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":"A3","manyB":[]}}}"###
        );
        // <<< End create test data

        // Cursor is B2. We skip 1, so B2 is not included. This makes:
        // A1 => [B3, B4, B5, B6]
        // A2 => [B3, B5, B7, B8]
        // A3 => []
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyModelA {
              id
              manyB(cursor: {
                id: "B2"
              }, skip: 1) {
                id
              }
            }
          }"#),
          @r###"{"data":{"findManyModelA":[{"id":"A1","manyB":[{"id":"B3"},{"id":"B4"},{"id":"B5"},{"id":"B6"}]},{"id":"A2","manyB":[{"id":"B3"},{"id":"B5"},{"id":"B7"},{"id":"B8"}]},{"id":"A3","manyB":[]}]}}"###
        );

        // Cursor is B2. We skip 1, so B2 is not included, and take the next 2. This makes:
        // A1 => [B3, B4]
        // A2 => [B3, B5]
        // A3 => []
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyModelA {
              id
              manyB(cursor: {
                id: "B2"
              }, skip: 1, take: 2) {
                id
              }
            }
          }"#),
          @r###"{"data":{"findManyModelA":[{"id":"A1","manyB":[{"id":"B3"},{"id":"B4"}]},{"id":"A2","manyB":[{"id":"B3"},{"id":"B5"}]},{"id":"A3","manyB":[]}]}}"###
        );

        // Cursor is B5. We skip 1, so B5 is not included, and take the previous 2 records. This makes:
        // A1 => [B3, B4]
        // A2 => [B2, B3]
        // A3 => []
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyModelA {
              id
              manyB(cursor: {
                id: "B5"
              }, skip: 1, take: -2, orderBy: { id: asc }) {
                id
              }
            }
          }"#),
          @r###"{"data":{"findManyModelA":[{"id":"A1","manyB":[{"id":"B3"},{"id":"B4"}]},{"id":"A2","manyB":[{"id":"B2"},{"id":"B3"}]},{"id":"A3","manyB":[]}]}}"###
        );

        Ok(())
    }

    // m:n relations, child is connected to many parents, using simple pagination
    // A1 <> B1, B2, B3, B4, B5, B6
    // A2 <> B2, B3, B5, B7, B8
    // A3
    // A many-to-many relationship with multiple connected children" should "return all items correctly with skip / take nested pagination
    #[connector_test(schema(simple_m2m))]
    async fn m2m_many_children_nested_skip_take(runner: Runner) -> TestResult<()> {
        // >>> Begin create test data
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModelA(
              data: {
                id: "A1"
                manyB: {
                  connectOrCreate: [
                    { where: { id: "B1" }, create: { id: "B1" } }
                    { where: { id: "B2" }, create: { id: "B2" } }
                    { where: { id: "B3" }, create: { id: "B3" } }
                    { where: { id: "B4" }, create: { id: "B4" } }
                    { where: { id: "B5" }, create: { id: "B5" } }
                    { where: { id: "B6" }, create: { id: "B6" } }
                  ]
                }
              }
            ) {
              id
              manyB {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":"A1","manyB":[{"id":"B1"},{"id":"B2"},{"id":"B3"},{"id":"B4"},{"id":"B5"},{"id":"B6"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModelA(
              data: {
                id: "A2"
                manyB: {
                  connectOrCreate: [
                    { where: { id: "B2" }, create: { id: "B2" } },
                    { where: { id: "B3" }, create: { id: "B3" } }
                    { where: { id: "B5" }, create: { id: "B5" } }
                    { where: { id: "B7" }, create: { id: "B7" } }
                    { where: { id: "B8" }, create: { id: "B8" } }
                  ]
                }
              }
            ) {
              id
              manyB {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":"A2","manyB":[{"id":"B2"},{"id":"B3"},{"id":"B5"},{"id":"B7"},{"id":"B8"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation{ createOneModelA(data: { id: "A3" }) { id manyB { id } } }"#),
          @r###"{"data":{"createOneModelA":{"id":"A3","manyB":[]}}}"###
        );
        // <<< End create test data

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findUniqueModelA(where: { id: "A1" }) {
              id
              manyB(skip: 1) {
                id
              }
            }
          }"#),
          @r###"{"data":{"findUniqueModelA":{"id":"A1","manyB":[{"id":"B2"},{"id":"B3"},{"id":"B4"},{"id":"B5"},{"id":"B6"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyModelA(orderBy: { id: asc }) {
              id
              manyB(skip: 1) {
                id
              }
            }
          }"#),
          @r###"{"data":{"findManyModelA":[{"id":"A1","manyB":[{"id":"B2"},{"id":"B3"},{"id":"B4"},{"id":"B5"},{"id":"B6"}]},{"id":"A2","manyB":[{"id":"B3"},{"id":"B5"},{"id":"B7"},{"id":"B8"}]},{"id":"A3","manyB":[]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findUniqueModelA(where: { id: "A1" }) {
              id
              manyB(skip: 1, take: 2) {
                id
              }
            }
          }"#),
          @r###"{"data":{"findUniqueModelA":{"id":"A1","manyB":[{"id":"B2"},{"id":"B3"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyModelA(orderBy: { id: asc }) {
              id
              manyB(skip: 1, take: 2) {
                id
              }
            }
          }"#),
          @r###"{"data":{"findManyModelA":[{"id":"A1","manyB":[{"id":"B2"},{"id":"B3"}]},{"id":"A2","manyB":[{"id":"B3"},{"id":"B5"}]},{"id":"A3","manyB":[]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findUniqueModelA(where: { id: "A1" }) {
              id
              manyB(skip: 1, take: -2, orderBy: { id: asc }) {
                id
              }
            }
          }"#),
          @r###"{"data":{"findUniqueModelA":{"id":"A1","manyB":[{"id":"B4"},{"id":"B5"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyModelA(orderBy: { id: asc }) {
              id
              manyB(skip: 1, take: -2, orderBy: { id: asc }) {
                id
              }
            }
          }"#),
          @r###"{"data":{"findManyModelA":[{"id":"A1","manyB":[{"id":"B4"},{"id":"B5"}]},{"id":"A2","manyB":[{"id":"B5"},{"id":"B7"}]},{"id":"A3","manyB":[]}]}}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{id: 1, t: "T1" middles:{create:[
          {id: 1, m: "M11" bottoms:{create:[
              {id: 1, b:"B111"}
              {id: 2, b:"B112"}
              {id: 3, b:"B113"}
          ]}
          },
          {id: 2, m: "M12" bottoms:{create:[
              {id: 4, b:"B121"}
              {id: 5, b:"B122"}
              {id: 6, b:"B123"}
          ]}
          },
          {id: 3, m: "M13" bottoms:{create:[
              {id: 7, b:"B131"}
              {id: 8, b:"B132"}
              {id: 9, b:"B133"}
          ]}
          }
       ]}}"#,
        )
        .await?;

        create_row(
            runner,
            r#"{id: 2, t: "T2" middles:{create:[
        {id: 4, m: "M21" bottoms:{create:[
            {id: 10, b:"B211"}
            {id: 11, b:"B212"}
            {id: 12, b:"B213"}
        ]}
        },
        {id: 5, m: "M22" bottoms:{create:[
            {id: 13, b:"B221"}
            {id: 14, b:"B222"}
            {id: 15, b:"B223"}
        ]}
        },
        {id: 6, m: "M23" bottoms:{create:[
            {id: 16, b:"B231"}
            {id: 17, b:"B232"}
            {id: 18, b:"B233"}
        ]}
        }
     ]}}"#,
        )
        .await?;

        create_row(
            runner,
            r#"{id: 3, t: "T3" middles:{create:[
          {id: 7, m: "M31" bottoms:{create:[
              {id: 19, b:"B311"}
              {id: 20, b:"B312"}
              {id: 21, b:"B313"}
          ]}
          },
          {id: 8, m: "M32" bottoms:{create:[
              {id: 22, b:"B321"}
              {id: 23, b:"B322"}
              {id: 24, b:"B323"}
          ]}
          },
          {id: 9, m: "M33" bottoms:{create:[
              {id: 25, b:"B331"}
              {id: 26, b:"B332"}
              {id: 27, b:"B333"}
          ]}
          }
       ]}}"#,
        )
        .await?;

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTop(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}
