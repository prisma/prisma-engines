fn main() {
    let prisma_schema = include_str!("../test-schemas/odoo.prisma");
    let source_file: psl::SourceFile = prisma_schema.into();
    let validated_schema = psl::validate(source_file);

    let now = std::time::Instant::now();
    let _ = schema::build(std::sync::Arc::new(validated_schema.into()), true);
    let elapsed = now.elapsed();

    println!("Elapsed: {:.2?}", elapsed);
}
