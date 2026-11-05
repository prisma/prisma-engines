use crate::introspection::datamodel_calculator::DatamodelCalculatorContext;

pub(crate) struct KingbaseOracleIntrospectionFlavour;

impl super::IntrospectionFlavour for KingbaseOracleIntrospectionFlavour {
    fn uses_pk_in_m2m_join_tables(&self, _ctx: &DatamodelCalculatorContext<'_>) -> bool {
        // Kingbase Oracle schema calculation emits the PostgreSQL-style `(A, B)` primary key
        // for Prisma implicit many-to-many join tables.
        true
    }
}
