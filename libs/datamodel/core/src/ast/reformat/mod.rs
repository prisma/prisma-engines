use crate::{parse_datamodel, render_datamodel_to};

pub struct Reformatter {}

impl Reformatter {
    pub fn reformat_to(input: &str, output: &mut dyn std::io::Write, _ident_width: usize) {
        let dml = parse_datamodel(&input).unwrap();
        render_datamodel_to(output, &dml).unwrap();
    }
}
