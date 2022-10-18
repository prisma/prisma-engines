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
            pub fn to_parts(&self) -> (&'static str, Vec<String>) {
                use $enumName::*;

                $crate::native_type_definition!(
                    @to_parts
                    self,
                    {}
                    $($variants)*
                )
            }

            #[allow(unused_variables)] // some impls (mongo) don't use the arguments param
            pub fn from_parts(
                name: &str,
                arguments: &[String],
                span: psl_core::parser_database::ast::Span,
                diagnostics: &mut psl_core::diagnostics::Diagnostics
            ) -> Option<Self> {
                use $enumName::*;

                $crate::native_type_definition!(
                    @from_parts
                    name, arguments, span, diagnostics,
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
                $variant(arg) => (stringify!($variant), <$param as psl_core::datamodel_connector::NativeTypeArguments>::to_parts(arg)),
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
                $variant => (stringify!($variant), Vec::new()),
            }
            $($tail)*
        }
    };

    // Base case
    (
        @from_parts
        $name:ident, $arguments:ident, $span:ident, $diagnostics:ident,
        { $($body:tt)* }
    ) => {
        match $name {
            $($body)*
            _ => {
                $diagnostics.push_error(psl_core::diagnostics::DatamodelError::new_native_type_parser_error($name, $span));
                None
            },
        }
    };

    (
        @from_parts
        $name:ident, $arguments:ident, $span:ident, $diagnostics:ident,
        { $($body:tt)* }
        $($(#[$variantDocs:meta])+)? $variant:ident ($params:ty) -> $($skip:ident)|*,
        $($tail:tt)*
    ) => {
        $crate::native_type_definition!(
            @from_parts
            $name, $arguments, $span, $diagnostics,
            {
                $($body)*
                name if name == stringify!($variant) => {
                    let args  = <$params as psl_core::datamodel_connector::NativeTypeArguments>::from_parts($arguments);
                    match args {
                        Some(args) => Some($variant(args)),
                        None => {
                            let rendered_args = format!("({})", $arguments.join(", "));
                            $diagnostics.push_error(psl_core::diagnostics::DatamodelError::new_value_parser_error(<$params as psl_core::datamodel_connector::NativeTypeArguments>::DESCRIPTION, &rendered_args, $span));
                            None
                        }
                    }
                },
            }
            $($tail)*
        )
    };

    (
        @from_parts
        $name:ident, $arguments:ident, $span:ident, $diagnostics:ident,
        { $($body:tt)* }
        $($(#[$variantDocs:meta])+)? $variant:ident -> $($skip:ident)|*,
        $($tail:tt)*
    ) => {
        $crate::native_type_definition!(
            @from_parts
            $name, $arguments, $span, $diagnostics,
            {
                $($body)*
                name if name == stringify!($variant) => Some($variant),
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
        use psl_core::datamodel_connector::NativeTypeConstructor;

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
                    name: stringify!($variant),
                    number_of_args: 0,
                    number_of_optional_args: 0,
                    prisma_types: &[$(psl_core::parser_database::ScalarType::$scalar),*],
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
                    name: stringify!($variant),
                    number_of_args: <$params as psl_core::datamodel_connector::NativeTypeArguments>::REQUIRED_ARGUMENTS_COUNT,
                    number_of_optional_args: <$params as psl_core::datamodel_connector::NativeTypeArguments>::OPTIONAL_ARGUMENTS_COUNT,
                    prisma_types: &[$(psl_core::parser_database::ScalarType::$scalar),*],
                },
            }
            $($tail)*
        }
    };
}
