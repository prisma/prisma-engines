//! A [tt-muncher macro](https://veykril.github.io/tlborm/decl-macros/patterns/tt-muncher.html) for
//! native type definitions.
//! It also uses the [internal rule pattern](https://veykril.github.io/tlborm/decl-macros/patterns/internal-rules.html) quite extensively.
//!
//! The first line in a native type definition should be the name for the native type enum,
//! followed by a semicolon, followed by lines mimicking enum variant declarations, but with a
//! trailing list of Prisma scalar types between curly braces for each variant. These are the
//! scalar types compatible with the native type.
//!
//! Example:
//!
//! ```ignore
//! crate::native_type_definition! {
//!    PostgresType;
//!    SmallInt {Int},
//!    VarChar(Option<u32>) {String},
//! }
//! ```
//!
//! This will define an enum of the form: `enum PostgresType { SmallInt, VarChar(Option<u32>) }`,
//! with `to_parts()` and `from_parts()` methods, as well as a `CONSTRUCTORS` constant containing
//! the corresponding `NativeTypeConstructor` values. The constructors will contain respectively
//! `Int` and `String` as compatible Prisma scalar types for the two native types.

#[macro_export]
macro_rules! native_type_definition {
    (
        $(#[$docs:meta])* $enumName:ident ;
        $($variants:tt)*
    ) => {
        $crate::native_type_definition!(
            @dataEnum
            $($docs)*;
            $enumName
            {}
            $($variants)*
        );

        impl $enumName {
            pub fn to_parts(&self) -> (&'static str, ::std::borrow::Cow<'static, [String]>) {
                use $enumName::*;

                $crate::native_type_definition!(
                    @to_parts
                    self,
                    {}
                    $($variants)*
                )
            }

            #[allow(unused_variables)] // some impls (mongo) don't use the arguments param
            pub fn from_parts<'a>(
                name: &'a str,
                arguments: &[String],
            ) -> ::std::result::Result<Self, $crate::datamodel_connector::NativeTypeParseError<'a>> {
                use $enumName::*;

                $crate::native_type_definition!(
                    @from_parts
                    name, arguments,
                    {}
                    $($variants)*
                )
            }
        }

        $crate::native_type_definition! {
            @nativeTypeConstructors
            {}
            $($variants)*
        }
    };

    // @dataEnum base case
    (
        @dataEnum
        $($docs:meta)*;
        $enumName:ident
        { $($body:tt)* }
    ) => {
        $(#[$docs])*
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub enum $enumName {
            $($body)*
        }
    };

    (
        @dataEnum
        $($docs:meta)*;
        $enumName:ident
        { $($body:tt)* }
        $($(#[$variantDocs:meta])+)? $variant:ident $(($param:ty))? -> $($skip:ident)|*,
        $($tail:tt)*
    ) => {
        $crate::native_type_definition! {
            @dataEnum
            $($docs)*;
            $enumName
            {
                $($body)*
                $($(#[$variantDocs])*)* $variant $(($param))*,
            }
            $($tail)*
        }
    };

    // Base case
    (
        @to_parts
        $self:ident,
        { $($body:tt)* }
    ) => {
        match $self {
            $($body)*
        }
    };

    (
        @to_parts
        $self:ident,
        { $($body:tt)* }
        $(#[$docs:meta])* $variant:ident ($param:ty) -> $($skip:ident)|*,
        $($tail:tt)*
    ) => {
        $crate::native_type_definition! {
            @to_parts
            $self,
            {
                $($body)*
                $variant(arg) => (stringify!($variant), <$param as $crate::datamodel_connector::NativeTypeArguments>::to_parts(arg).into()),
            }
            $($tail)*
        }
    };

    (
        @to_parts
        $self:ident,
        { $($body:tt)* }
        $(#[$docs:meta])* $variant:ident -> $($skip:ident)|*,
        $($tail:tt)*
    ) => {
        $crate::native_type_definition! {
            @to_parts
            $self,
            {
                $($body)*
                $variant => (stringify!($variant), Vec::new().into()),
            }
            $($tail)*
        }
    };

    // Base case
    (
        @from_parts
        $name:ident, $arguments:ident,
        { $($body:tt)* }
    ) => {
        match $name {
            $($body)*
            _ => Err($crate::datamodel_connector::NativeTypeParseError::UnknownType { name: $name }),
        }
    };

    (
        @from_parts
        $name:ident, $arguments:ident,
        { $($body:tt)* }
        $($(#[$variantDocs:meta])+)? $variant:ident ($params:ty) -> $($skip:ident)|*,
        $($tail:tt)*
    ) => {
        $crate::native_type_definition!(
            @from_parts
            $name, $arguments,
            {
                $($body)*
                name if name == stringify!($variant) => {
                    let args  = <$params as $crate::datamodel_connector::NativeTypeArguments>::from_parts($arguments);
                    match args {
                        Some(args) => Ok($variant(args)),
                        None => {
                            let rendered_args = format!("({})", $arguments.join(", "));
                            Err($crate::datamodel_connector::NativeTypeParseError::InvalidArgs { expected: <$params as $crate::datamodel_connector::NativeTypeArguments>::DESCRIPTION, found: rendered_args })
                        }
                    }
                },
            }
            $($tail)*
        )
    };

    (
        @from_parts
        $name:ident, $arguments:ident,
        { $($body:tt)* }
        $($(#[$variantDocs:meta])+)? $variant:ident -> $($skip:ident)|*,
        $($tail:tt)*
    ) => {
        $crate::native_type_definition!(
            @from_parts
            $name, $arguments,
            {
                $($body)*
                name if name == stringify!($variant) => Ok($variant),
            }
            $($tail)*
        )
    };

    // Base case
    (
        @nativeTypeConstructors
        { $($body:tt)* }
    ) => {
        #[allow(unused_imports)]
        use $crate::datamodel_connector::NativeTypeConstructor;

        pub(crate) const CONSTRUCTORS: &[NativeTypeConstructor] = &[
            $( $body )*
        ];
    };

    (
        @nativeTypeConstructors
        { $($body:tt)* }
        $($(#[$variantDocs:meta])+)? $variant:ident -> $($scalar:ident)|*,
        $($tail:tt)*
    ) => {
        $crate::native_type_definition! {
            @nativeTypeConstructors
            {
                $($body)*
                NativeTypeConstructor {
                    name: ::std::borrow::Cow::Borrowed(stringify!($variant)),
                    number_of_args: 0,
                    number_of_optional_args: 0,
                    allowed_types: ::std::borrow::Cow::Borrowed(&[$($crate::datamodel_connector::AllowedType::plain($crate::parser_database::ScalarFieldType::BuiltInScalar($crate::parser_database::ScalarType::$scalar))),*]),
                },
            }
            $($tail)*
        }
    };

    (
        @nativeTypeConstructors
        { $($body:tt)* }
        $($(#[$variantDocs:meta])+)? $variant:ident ($params:ty) -> $($scalar:ident)|*,
        $($tail:tt)*
    ) => {
        $crate::native_type_definition! {
            @nativeTypeConstructors
            {
                $($body)*
                NativeTypeConstructor {
                    name: ::std::borrow::Cow::Borrowed(stringify!($variant)),
                    number_of_args: <$params as $crate::datamodel_connector::NativeTypeArguments>::REQUIRED_ARGUMENTS_COUNT,
                    number_of_optional_args: <$params as $crate::datamodel_connector::NativeTypeArguments>::OPTIONAL_ARGUMENTS_COUNT,
                    allowed_types: ::std::borrow::Cow::Borrowed(&[$($crate::datamodel_connector::AllowedType::plain($crate::parser_database::ScalarFieldType::BuiltInScalar($crate::parser_database::ScalarType::$scalar))),*]),
                },
            }
            $($tail)*
        }
    };
}
