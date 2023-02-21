use std::process::Command;

pub fn main() {
    let compiler = cc::Build::new().get_compiler().path().to_owned();
    let preprocessed = String::from_utf8(
        Command::new(compiler)
            .args(["-E", "src/nope.S"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .replace("__NL__", "\n");
    std::fs::write("src/nope-pp.s", preprocessed).unwrap();
    cc::Build::new().file("src/nope-pp.s").compile("nope_asm");
}
