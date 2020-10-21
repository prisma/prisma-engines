use crate::common::*;
use datamodel::{ast, diagnostics::DatamodelError};

#[test]
fn should_fail_on_native_type_text_with_unique_attribute() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url = "mysql://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int    @id
            bigInt String @db.Text @unique
        }

        model User {
            id        Int     @default(autoincrement()) @id
            firstname String @db.Text
            lastname  String @db.Text
            @@unique([firstname, lastname])
        }
    "#;

    let error = parse_error(dml);

    error.assert_length(2);

    error.assert_is_at(
        0,
        DatamodelError::new_connector_error("Native type Text can not be unique in MySQL.", ast::Span::new(277, 308)),
    );
    error.assert_is_at(
        1,
        DatamodelError::new_connector_error("Native type Text can not be unique in MySQL.", ast::Span::new(327, 529)),
    );
}

#[test]
fn should_fail_on_native_type_long_text_with_unique_attribute() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url = "mysql://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int    @id
            bigInt String @db.LongText @unique
        }

        model User {
            id        Int     @default(autoincrement()) @id
            firstname String @db.LongText
            lastname  String @db.LongText
            @@unique([firstname, lastname])
        }
    "#;

    let error = parse_error(dml);

    error.assert_length(2);

    error.assert_is_at(
        0,
        DatamodelError::new_connector_error(
            "Native type LongText can not be unique in MySQL.",
            ast::Span::new(277, 312),
        ),
    );
    error.assert_is_at(
        1,
        DatamodelError::new_connector_error(
            "Native type LongText can not be unique in MySQL.",
            ast::Span::new(331, 541),
        ),
    );
}

#[test]
fn should_fail_on_native_type_medium_text_with_unique_attribute() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url = "mysql://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int    @id
            bigInt String @db.MediumText @unique
        }

        model User {
            id        Int     @default(autoincrement()) @id
            firstname String @db.MediumText
            lastname  String @db.MediumText
            @@unique([firstname, lastname])
        }
    "#;

    let error = parse_error(dml);

    error.assert_length(2);

    error.assert_is_at(
        0,
        DatamodelError::new_connector_error(
            "Native type MediumText can not be unique in MySQL.",
            ast::Span::new(277, 314),
        ),
    );
    error.assert_is_at(
        1,
        DatamodelError::new_connector_error(
            "Native type MediumText can not be unique in MySQL.",
            ast::Span::new(333, 547),
        ),
    );
}

#[test]
fn should_fail_on_native_type_tiny_text_with_unique_attribute() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url = "mysql://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int    @id
            bigInt String @db.TinyText @unique
        }

        model User {
            id        Int     @default(autoincrement()) @id
            firstname String @db.TinyText
            lastname  String @db.TinyText
            @@unique([firstname, lastname])
        }
    "#;

    let error = parse_error(dml);

    error.assert_length(2);

    error.assert_is_at(
        0,
        DatamodelError::new_connector_error(
            "Native type TinyText can not be unique in MySQL.",
            ast::Span::new(277, 312),
        ),
    );
    error.assert_is_at(
        1,
        DatamodelError::new_connector_error(
            "Native type TinyText can not be unique in MySQL.",
            ast::Span::new(331, 541),
        ),
    );
}

#[test]
fn should_fail_on_native_type_blob_with_unique_attribute() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url = "mysql://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int   @id
            bigInt Bytes @db.Blob @unique
        }

        model User {
            id        Int     @default(autoincrement()) @id
            firstname Bytes @db.Blob
            lastname  Bytes @db.Blob
            @@unique([firstname, lastname])
        }
    "#;

    let error = parse_error(dml);

    error.assert_length(2);

    error.assert_is_at(
        0,
        DatamodelError::new_connector_error("Native type Blob can not be unique in MySQL.", ast::Span::new(276, 306)),
    );
    error.assert_is_at(
        1,
        DatamodelError::new_connector_error("Native type Blob can not be unique in MySQL.", ast::Span::new(325, 525)),
    );
}

#[test]
fn should_fail_on_native_type_tiny_blob_with_unique_attribute() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url = "mysql://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int    @id
            bigInt Bytes @db.TinyBlob @unique
        }

        model User {
            id        Int     @default(autoincrement()) @id
            firstname Bytes @db.TinyBlob
            lastname  Bytes @db.TinyBlob
            @@unique([firstname, lastname])
        }
    "#;

    let error = parse_error(dml);

    error.assert_length(2);

    error.assert_is_at(
        0,
        DatamodelError::new_connector_error(
            "Native type TinyBlob can not be unique in MySQL.",
            ast::Span::new(277, 311),
        ),
    );
    error.assert_is_at(
        1,
        DatamodelError::new_connector_error(
            "Native type TinyBlob can not be unique in MySQL.",
            ast::Span::new(330, 538),
        ),
    );
}

#[test]
fn should_fail_on_native_type_medium_blob_with_unique_attribute() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url = "mysql://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int    @id
            bigInt Bytes @db.MediumBlob @unique
        }

        model User {
            id        Int     @default(autoincrement()) @id
            firstname Bytes @db.MediumBlob
            lastname  Bytes @db.MediumBlob
            @@unique([firstname, lastname])
        }
    "#;

    let error = parse_error(dml);

    error.assert_length(2);

    error.assert_is_at(
        0,
        DatamodelError::new_connector_error(
            "Native type MediumBlob can not be unique in MySQL.",
            ast::Span::new(277, 313),
        ),
    );
    error.assert_is_at(
        1,
        DatamodelError::new_connector_error(
            "Native type MediumBlob can not be unique in MySQL.",
            ast::Span::new(332, 544),
        ),
    );
}

#[test]
fn should_fail_on_native_type_long_blob_with_unique_attribute() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url      = "mysql://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int   @id
            bigInt Bytes @db.LongBlob @unique
        }

        model User {
            id        Int     @default(autoincrement()) @id
            firstname Bytes @db.LongBlob
            lastname  Bytes @db.LongBlob
            @@unique([firstname, lastname])
        }
    "#;

    let error = parse_error(dml);

    error.assert_length(2);

    error.assert_is_at(
        0,
        DatamodelError::new_connector_error(
            "Native type LongBlob can not be unique in MySQL.",
            ast::Span::new(281, 315),
        ),
    );
    error.assert_is_at(
        1,
        DatamodelError::new_connector_error(
            "Native type LongBlob can not be unique in MySQL.",
            ast::Span::new(334, 542),
        ),
    );
}

#[test]
fn should_fail_on_native_type_decimal_when_scale_is_bigger_than_precision() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url      = "mysql://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int   @id
            dec Decimal @db.Decimal(2, 4)
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "The scale must not be larger than the precision for the Decimal native type in MySQL.",
        ast::Span::new(281, 311),
    ));
}

#[test]
fn should_fail_on_native_type_numeric_when_scale_is_bigger_than_precision() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url      = "mysql://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int   @id
            dec Decimal @db.Numeric(2, 4)
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "The scale must not be larger than the precision for the Numeric native type in MySQL.",
        ast::Span::new(281, 311),
    ));
}
