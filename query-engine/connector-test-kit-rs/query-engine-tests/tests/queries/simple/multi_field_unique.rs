use query_engine_tests::*;

#[test_suite]
mod multi_field_unique {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query};

    fn two_field_unique() -> String {
        let schema = indoc! {
            r#"model User {
                #id(id, Int, @id)
                FirstName String
                LastName  String

                @@unique([FirstName, LastName])
            }"#
        };

        schema.to_owned()
    }

    fn aliased() -> String {
        let schema = indoc! {
            r#"model User {
                #id(id, Int, @id)
                FirstName String
                LastName  String

                @@unique([FirstName, LastName], name: "full_name")
            }"#
        };

        schema.to_owned()
    }

    fn single_multi() -> String {
        let schema = indoc! {
            r#"model User {
                #id(id, Int, @id)
                uniq String

                @@unique([uniq])
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(two_field_unique))]
    async fn simple(runner: Runner) -> TestResult<()> {
        create_user(&runner, r#"{ id: 1, FirstName: "Matt", LastName: "Eagle" }"#).await?;
        create_user(&runner, r#"{ id: 2, FirstName: "Hans", LastName: "Wurst" }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
          {
                findUniqueUser(where: {
                    FirstName_LastName: {
                        FirstName: "Hans"
                        LastName: "Wurst"
                    }
                }) {
                    id
                }
          }
          "# }),
          @r###"{"data":{"findUniqueUser":{"id":2}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(two_field_unique))]
    async fn non_existant_user(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
          {
                findUniqueUser(where: {
                    FirstName_LastName: {
                        FirstName: "Foo"
                        LastName: "Bar"
                    }
                }) {
                    id
                }
          }
          "# }),
          @r###"{"data":{"findUniqueUser":null}}"###
        );

        Ok(())
    }

    #[connector_test(schema(two_field_unique))]
    async fn incomplete_where(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            indoc! { r#"
                {
                    findUniqueUser(where: {
                        FirstName_LastName: {
                            FirstName: "Foo"
                        }
                    }) {
                        id
                    }
            }
            "# },
            2012,
            "A value is required but not set"
        );

        Ok(())
    }

    #[connector_test(schema(aliased))]
    async fn aliased_index(runner: Runner) -> TestResult<()> {
        create_user(&runner, r#"{ id: 1, FirstName: "Matt", LastName: "Eagle" }"#).await?;
        create_user(&runner, r#"{ id: 2, FirstName: "Hans", LastName: "Wurst" }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
          {
                findUniqueUser(where: {
                    full_name: {
                        FirstName: "Hans"
                        LastName: "Wurst"
                    }
                }) {
                    id
                }
          }
          "# }),
          @r###"{"data":{"findUniqueUser":{"id":2}}}"###
        );

        Ok(())
    }

    fn many_unique_fields() -> String {
        let schema = indoc! {
            r#"model User {
                #id(id, Int, @id)
                a String
                b String
                c String
                d String
                e String
                f String
                g String
                h String
                i String
                j String
                k String
                l String
                m String
                n String
                o String
                p String
                q String
                r String
                s String
                t String
                u String
                v String
                w String
                x String
                y String
                z String

                @@unique([a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w, x, y, z])
              }
              "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(many_unique_fields), exclude(MySQL, Vitess))]
    async fn ludicrous_fields(runner: Runner) -> TestResult<()> {
        create_user(
            &runner,
            indoc! { r#"{
                id: 1,
                a: "test"
                b: "test"
                c: "test"
                d: "test"
                e: "test"
                f: "test"
                g: "test"
                h: "test"
                i: "test"
                j: "test"
                k: "test"
                l: "test"
                m: "test"
                n: "test"
                o: "test"
                p: "test"
                q: "test"
                r: "test"
                s: "test"
                t: "test"
                u: "test"
                v: "test"
                w: "test"
                x: "test"
                y: "test"
                z: "test"
            }"# },
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
          {
            findUniqueUser(where: {a_b_c_d_e_f_g_h_i_j_k_l_m_n_o_p_q_r_s_t_u_v_w_x_y_z: {
              a: "test"
              b: "test"
              c: "test"
              d: "test"
              e: "test"
              f: "test"
              g: "test"
              h: "test"
              i: "test"
              j: "test"
              k: "test"
              l: "test"
              m: "test"
              n: "test"
              o: "test"
              p: "test"
              q: "test"
              r: "test"
              s: "test"
              t: "test"
              u: "test"
              v: "test"
              w: "test"
              x: "test"
              y: "test"
              z: "test"
            }}){
              id
            }
          }
          "# }),
          @r###"{"data":{"findUniqueUser":{"id":1}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(single_multi))]
    async fn single_field_multi_unique(runner: Runner) -> TestResult<()> {
        create_user(&runner, r#"{ id: 1, uniq: "test" }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"{ findUniqueUser(where: { uniq: "test" }) { id }}"# }),
          @r###"{"data":{"findUniqueUser":{"id":1}}}"###
        );

        Ok(())
    }

    async fn create_user(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneUser(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();

        Ok(())
    }
}
