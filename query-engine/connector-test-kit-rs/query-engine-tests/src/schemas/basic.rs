use indoc::indoc;

/// Most basic datamodel containing only a model with ID
/// for the most rudimentary testing.
pub fn generic() -> String {
    let schema = indoc! {
        "model TestModel {
            #id(id, Int, @id)
            field String?
        }"
    };

    schema.to_owned()
}

/// User model with some basic fields and unique constraints.
pub fn user() -> String {
    let schema = indoc! {
        "model User {
            #id(id, Int, @id)
            first_name String
            last_name  String
            email      String    @unique
            birthday   DateTime?

            @@unique([first_name, last_name])
        }"
    };

    schema.to_owned()
}
