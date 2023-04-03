fn main() {
    let prisma_schema = include_str!("../test-schemas/odoo.prisma");
    let source_file: psl::SourceFile = prisma_schema.into();
    let validated_schema = std::sync::Arc::new(psl::validate(source_file));
    let idm = prisma_models::convert(validated_schema);

    let now = std::time::Instant::now();
    let _ = schema_builder::build(idm.clone(), true);
    let elapsed = now.elapsed();

    println!("Elapsed: {:.2?}", elapsed);
}
