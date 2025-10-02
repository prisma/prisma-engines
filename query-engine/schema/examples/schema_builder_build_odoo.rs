fn main() {
    let prisma_schema = include_str!("../test-schemas/odoo.prisma");
    let source_file: psl::SourceFile = prisma_schema.into();
    let validated_schema = std::sync::Arc::new(psl::validate_without_extensions(source_file));

    let now = std::time::Instant::now();
    let _ = schema::build(validated_schema, true);
    let elapsed = now.elapsed();

    println!("Elapsed: {elapsed:.2?}");
}
