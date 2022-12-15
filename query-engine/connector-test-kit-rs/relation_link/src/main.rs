use setup::{schema_with_relation, RelationField};

fn main() {
    let on_parent = RelationField::ToOneOpt { child: false };
    let on_child = RelationField::ToOneOpt { child: true };

    let (datamodels, _) = schema_with_relation(&on_parent, &on_child, false);

    for d in &datamodels {
        println!("{}", d.datamodel());
        println!()
    }
}
