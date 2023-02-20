use std::process::Command;

pub fn main() {
    let preprocessed = String::from_utf8(Command::new("gcc").args(["-E", "src/nope.S"]).output().unwrap().stdout)
        .unwrap()
        .replace("__NL__", "\n");
    std::fs::write("src/nope-pp.s", preprocessed).unwrap();
    cc::Build::new().file("src/nope-pp.s").compile("nope_asm");
}
