pub fn main() {
    cc::Build::new().file("src/nope.S").compile("nope_asm");
}
