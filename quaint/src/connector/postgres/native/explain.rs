#[derive(serde::Deserialize, Debug)]
#[serde(untagged)]
pub(crate) enum Explain {
    // NOTE: the returned JSON may not contain a `plan` field, for example, with `CALL` statements:
    // https://github.com/launchbadge/sqlx/issues/1449
    //
    // In this case, we should just fall back to assuming all is nullable.
    //
    // It may also contain additional fields we don't care about, which should not break parsing:
    // https://github.com/launchbadge/sqlx/issues/2587
    // https://github.com/launchbadge/sqlx/issues/2622
    Plan {
        #[serde(rename = "Plan")]
        plan: Plan,
    },

    // This ensures that parsing never technically fails.
    //
    // We don't want to specifically expect `"Utility Statement"` because there might be other cases
    // and we don't care unless it contains a query plan anyway.
    Other(serde::de::IgnoredAny),
}

#[derive(serde::Deserialize, Debug)]
pub(crate) struct Plan {
    #[serde(rename = "Join Type")]
    pub(crate) join_type: Option<String>,
    #[serde(rename = "Parent Relationship")]
    pub(crate) parent_relation: Option<String>,
    #[serde(rename = "Output")]
    pub(crate) output: Option<Vec<String>>,
    #[serde(rename = "Plans")]
    pub(crate) plans: Option<Vec<Plan>>,
}

pub(crate) fn visit_plan(plan: &Plan, outputs: &[String], nullables: &mut Vec<Option<bool>>) {
    if let Some(plan_outputs) = &plan.output {
        // all outputs of a Full Join must be marked nullable
        // otherwise, all outputs of the inner half of an outer join must be marked nullable
        if plan.join_type.as_deref() == Some("Full") || plan.parent_relation.as_deref() == Some("Inner") {
            for output in plan_outputs {
                if let Some(i) = outputs.iter().position(|o| o == output) {
                    // N.B. this may produce false positives but those don't cause runtime errors
                    nullables[i] = Some(true);
                }
            }
        }
    }

    if let Some(plans) = &plan.plans {
        if let Some("Left") | Some("Right") = plan.join_type.as_deref() {
            for plan in plans {
                visit_plan(plan, outputs, nullables);
            }
        }
    }
}
