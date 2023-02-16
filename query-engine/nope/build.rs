pub fn main() {
    cc::Build::new().file("src/nope.s").compile("nope_asm");
}
