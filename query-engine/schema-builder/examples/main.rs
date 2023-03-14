use std::sync::Arc;

fn main() {
    use std::time::Instant;

    let prisma_schema = include_str!("../benches/odoo.prisma");
    let source_file: psl::SourceFile = prisma_schema.into();
    let schema = Arc::new(psl::validate(source_file.clone()));
    let idm = prisma_models::convert(schema);

    let now = Instant::now();
    schema_builder::build(idm.clone(), true);
    let elapsed = now.elapsed();

    println!("Elapsed: {:.2?}", elapsed);
}
