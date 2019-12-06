use clap::{App, Arg};
pub fn clap_app() -> clap::App<'static, 'static> {
    App::new("Prisma Introspection Engine")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::with_name("version")
                .long("version")
                .help("Prints the server commit ID")
                .takes_value(false)
                .required(false),
        )
}
