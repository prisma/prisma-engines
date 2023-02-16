extern "C" {
    fn nope_nops();
}

#[inline(never)]
pub fn nops() {
    unsafe { nope_nops() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        nops();
    }
}
