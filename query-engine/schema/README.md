# schema

This crate contains the logic responsible for building a query schema from a Prisma datamodel (presented as a `query_structure::InternalDataModel`).

## Benchmarks

The benchmarks are defined in the `benches/` folder. They measure different steps of schema building (not only those defined in this crate, which is the last step), with Prisma schemas of different sizes.

You can run them like this:

```bash
$ cargo bench
```
