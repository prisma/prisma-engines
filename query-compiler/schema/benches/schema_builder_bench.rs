use codspeed_criterion_compat::{Criterion, black_box, criterion_group, criterion_main};

const SMALL: (&str, &str) = ("small", include_str!("../test-schemas/standupbot.prisma"));
const MEDIUM: (&str, &str) = ("medium", include_str!("../test-schemas/noalyss_folder.prisma"));
const LARGE: (&str, &str) = ("large", include_str!("../test-schemas/odoo.prisma"));

fn criterion_benchmark(c: &mut Criterion) {
    for (name, prisma_schema) in [SMALL, MEDIUM, LARGE] {
        let source_file: psl::SourceFile = prisma_schema.into();

        c.bench_function(&format!("psl::validate ({name})"), |b| {
            b.iter(|| black_box(psl::validate_without_extensions(source_file.clone())))
        });

        let validated_schema = std::sync::Arc::new(psl::validate_without_extensions(source_file));

        c.bench_function(&format!("schema_builder::build ({name})"), |b| {
            b.iter(|| black_box(schema::build(validated_schema.clone(), true)));
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
