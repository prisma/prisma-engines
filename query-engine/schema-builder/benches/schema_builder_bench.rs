use criterion::{black_box, criterion_group, criterion_main, Criterion};

const SMALL: (&str, &str) = ("small", include_str!("./standupbot.prisma"));
const MEDIUM: (&str, &str) = ("medium", include_str!("./noalyss_folder.prisma"));
const LARGE: (&str, &str) = ("large", include_str!("./odoo.prisma"));

pub fn criterion_benchmark(c: &mut Criterion) {
    for (name, prisma_schema) in [SMALL, MEDIUM, LARGE] {
        let source_file: psl::SourceFile = prisma_schema.into();

        c.bench_function(&format!("psl::validate ({name})"), |b| {
            b.iter(|| black_box(psl::validate(source_file.clone())))
        });

        let validated_schema = std::sync::Arc::new(psl::validate(source_file));
        let idm = prisma_models::convert(validated_schema);

        c.bench_function(&format!("schema_builder::build ({name})"), |b| {
            b.iter(|| black_box(schema_builder::build(idm.clone(), true)));
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
