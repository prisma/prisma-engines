mod old;

use crate::ast::reformat::old::ReformatterOld;
use crate::common::WritableString;
use crate::{
    ast, parse_datamodel_and_ignore_env_errors, parse_schema_ast, render_schema_ast_to, validator::LowerDmlToAst,
};

pub struct Reformatter {}

impl Reformatter {
    pub fn reformat_to(input: &str, output: &mut dyn std::io::Write, _ident_width: usize) {
        //        // the AST contains the datasources, generators, type aliases that are missing in the dml
        //        // it also contains all the original positions within the file
        //        let mut ast = parse_schema_ast(&input).unwrap();
        //        let dml = parse_datamodel_and_ignore_env_errors(&input).unwrap();
        //
        //        for top in ast.tops.iter_mut() {
        //            match top {
        //                ast::Top::Model(model) => {
        //                    let lowerer = LowerDmlToAst::new();
        //                    let dml_model = dml.find_model(&model.name.name).unwrap();
        //                    let new_model = lowerer.lower_model(&dml_model, &dml).unwrap();
        //                    std::mem::replace(top, ast::Top::Model(new_model));
        //                }
        //                _ => {}
        //            }
        //        }
        //
        //        render_schema_ast_to(output, &ast, 2);
        let reformatter = ReformatterOld::new(input);
        reformatter.reformat_to(output, _ident_width)
    }

    pub fn reformat_to_string(input: &str) -> String {
        let mut result = WritableString::new();
        Reformatter::reformat_to(input, &mut result, 0);
        result.into()
    }
}
