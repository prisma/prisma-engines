use super::*;

#[derive(Debug, Clone)]
pub enum RelationReference<'a> {
    SimpleChildId(&'a RelationField),
    SimpleParentId(&'a RelationField),
    CompoundParentId(&'a RelationField),
    CompoundChildId(&'a RelationField),
    ParentReference(&'a RelationField),
    CompoundParentReference(&'a RelationField),
    ChildReference(&'a RelationField),
    CompoundChildReference(&'a RelationField),
    IdReference,
    NoRef,
}

impl RelationReference<'_> {
    pub fn render(&self) -> String {
        match self {
            RelationReference::SimpleChildId(rf) => self.render_simple_child_id(rf),
            RelationReference::SimpleParentId(rf) => self.render_simple_parent_id(rf),
            RelationReference::CompoundParentId(rf) => self.render_compound_parent_id(rf),
            RelationReference::CompoundChildId(rf) => self.render_compound_child_id(rf),
            RelationReference::ParentReference(rf) => self.render_parent_ref(rf),
            RelationReference::CompoundParentReference(rf) => self.render_compound_parent_ref(rf),
            RelationReference::ChildReference(rf) => self.render_child_ref(rf),
            RelationReference::CompoundChildReference(rf) => self.render_compound_child_ref(rf),
            RelationReference::NoRef => "".to_string(),
            RelationReference::IdReference => "@relation(references: [id])".to_string(),
        }
    }

    fn render_simple_child_id(&self, rf: &RelationField) -> String {
        match rf.is_list() {
            true => "@relation(references: [id])".to_string(),
            false => format!(
                "@relation(fields: [childId], references: [id]) \n childId String{}",
                rf.optional_suffix()
            ),
        }
    }

    fn render_simple_parent_id(&self, rf: &RelationField) -> String {
        match rf.is_list() {
            true => "@relation(references: [id])".to_string(),
            false => format!(
                "@relation(fields: [parentId], references: [id]) \n parentId String{}",
                rf.optional_suffix()
            ),
        }
    }

    fn render_compound_parent_id(&self, rf: &RelationField) -> String {
        match rf.is_list() {
            true => "@relation(references: [id_1, id_2])".to_string(),
            false => format!(
                "@relation(fields: [parent_id_1, parent_id_2], references: [id_1, id_2]) \n parent_id_1 String{}\n parent_id_2 String{}",
                rf.optional_suffix(),
                rf.optional_suffix()
            ),
        }
    }

    fn render_compound_child_id(&self, rf: &RelationField) -> String {
        match rf.is_list() {
            true => "@relation(references: [id_1, id_2])".to_string(),
            false => format!(
                "@relation(fields: [child_id_1, child_id_2], references: [id_1, id_2])\n child_id_1 String{}\n child_id_2 String{}",
                rf.optional_suffix(),
                rf.optional_suffix()
            ),
        }
    }

    fn render_parent_ref(&self, rf: &RelationField) -> String {
        match rf.is_list() {
            true => "@relation(references: [p])".to_string(),
            false => format!(
                "@relation(fields: [parentRef], references: [p]) \n parentRef String{}",
                rf.optional_suffix()
            ),
        }
    }

    fn render_compound_parent_ref(&self, rf: &RelationField) -> String {
        match rf.is_list() {
            true => "@relation(references: [p_1, p_2])".to_string(),
            false => format!(
                "@relation(fields: [parent_p_1, parent_p_2], references: [p_1, p_2])\nparent_p_1 String{}\n parent_p_2 String{}",
                rf.optional_suffix(),
                rf.optional_suffix()
            ),
        }
    }

    fn render_child_ref(&self, rf: &RelationField) -> String {
        match rf.is_list() {
            true => "@relation(references: [c])".to_string(),
            false => format!(
                "@relation(fields:[parent_c], references: [c])\nparent_c String{}",
                rf.optional_suffix()
            ),
        }
    }

    fn render_compound_child_ref(&self, rf: &RelationField) -> String {
        //"@relation(references: [c_1, c_2]) @map([\"child_c_1\", \"child_c_2\"])"
        match rf.is_list() {
            true => "@relation(references: [c_1, c_2])".to_string(),
            false => format!(
                "@relation(fields: [child_c_1, child_c_2], references: [c_1, c_2])\n child_c_1 String{}\n child_c_2 String{}",
                rf.optional_suffix(),
                rf.optional_suffix()
            ),
        }
    }
}

pub fn common_parent_references(rf: &RelationField) -> Vec<RelationReference> {
    vec![
        RelationReference::ParentReference(rf),
        RelationReference::CompoundParentReference(rf),
    ]
}

pub fn common_child_references(rf: &RelationField) -> Vec<RelationReference> {
    vec![
        RelationReference::ChildReference(rf),
        RelationReference::CompoundChildReference(rf),
    ]
}

pub fn child_references<'a>(
    simple: bool,
    parent_id: &Identifier,
    on_parent: &'a RelationField,
    on_child: &'a RelationField,
) -> Vec<RelationReference<'a>> {
    if simple {
        simple_child_references(parent_id, on_parent, on_child)
    } else {
        full_child_references(parent_id, on_parent, on_child)
    }
}

pub fn simple_child_references<'a>(
    parent_id: &Identifier,
    on_parent: &'a RelationField,
    on_child: &'a RelationField,
) -> Vec<RelationReference<'a>> {
    match *parent_id {
        _ if on_child.is_list() && !on_parent.is_list() => vec![(RelationReference::NoRef)],
        Identifier::Simple if on_child.is_to_one_opt() && on_parent.is_to_one_opt() => {
            vec![RelationReference::SimpleParentId(on_child), RelationReference::NoRef]
        }
        Identifier::Simple => vec![RelationReference::SimpleParentId(on_child)],
        Identifier::Compound => vec![RelationReference::CompoundParentId(on_child)],
        Identifier::None => vec![RelationReference::ParentReference(on_child)],
    }
}

pub fn full_child_references<'a>(
    parent_id: &Identifier,
    on_parent: &'a RelationField,
    on_child: &'a RelationField,
) -> Vec<RelationReference<'a>> {
    let is_m2m = on_parent.is_list() && on_child.is_list();

    if !is_m2m {
        match *parent_id {
            _ if on_child.is_list() && !on_parent.is_list() => vec![RelationReference::NoRef],
            Identifier::Simple if on_child.is_to_one_opt() && on_parent.is_to_one_opt() => {
                vec![RelationReference::SimpleParentId(on_child), RelationReference::NoRef]
                    .clone_append(&mut common_parent_references(on_child))
            }
            Identifier::Simple => {
                vec![RelationReference::SimpleParentId(on_child)].clone_append(&mut common_parent_references(on_child))
            }
            Identifier::Compound => vec![RelationReference::CompoundParentId(on_child)]
                .clone_append(&mut common_parent_references(on_child)),
            _ => common_parent_references(on_child),
        }
    } else {
        match *parent_id {
            Identifier::Simple => vec![RelationReference::SimpleParentId(on_child)],
            Identifier::Compound => vec![RelationReference::CompoundParentId(on_child)],
            _ => vec![RelationReference::ParentReference(on_child)],
        }
    }
}

pub fn parent_references<'a>(
    simple: bool,
    child_id: &Identifier,
    child_reference: &'a RelationReference,
    on_parent: &'a RelationField,
    on_child: &'a RelationField,
) -> Vec<RelationReference<'a>> {
    if simple {
        simple_parent_references(child_id, child_reference, on_parent, on_child)
    } else {
        full_parent_references(child_id, child_reference, on_parent, on_child)
    }
}

pub fn simple_parent_references<'a>(
    child_id: &Identifier,
    child_reference: &'a RelationReference,
    on_parent: &'a RelationField,
    on_child: &'a RelationField,
) -> Vec<RelationReference<'a>> {
    let is_m2m = on_parent.is_list() && on_child.is_list();

    match *child_id {
        _ if child_reference.render() != RelationReference::NoRef.render() && !is_m2m => vec![RelationReference::NoRef],
        Identifier::Simple => vec![RelationReference::SimpleChildId(on_parent)],
        Identifier::Compound => vec![RelationReference::CompoundChildId(on_parent)],
        Identifier::None => vec![RelationReference::ChildReference(on_parent)],
    }
}

pub fn full_parent_references<'a>(
    child_id: &Identifier,
    child_reference: &'a RelationReference,
    on_parent: &'a RelationField,
    on_child: &'a RelationField,
) -> Vec<RelationReference<'a>> {
    let is_m2m = on_parent.is_list() && on_child.is_list();

    if !is_m2m {
        match (child_id, child_reference) {
            (_, _) if on_parent.is_list() && !on_child.is_list() => vec![RelationReference::NoRef],
            (&Identifier::Simple, RelationReference::NoRef) => {
                vec![RelationReference::SimpleChildId(on_parent)].clone_append(&mut common_child_references(on_parent))
            }
            (&Identifier::Simple, _) if on_parent.is_list() && on_child.is_list() => {
                let mut refs = vec![RelationReference::SimpleChildId(on_parent)]
                    .clone_append(&mut common_child_references(on_parent));
                refs.push(RelationReference::NoRef);

                refs
            }
            (&Identifier::Simple, _) => vec![RelationReference::NoRef],
            (&Identifier::Compound, RelationReference::NoRef) => vec![RelationReference::CompoundChildId(on_parent)]
                .clone_append(&mut common_child_references(on_parent)),
            (&Identifier::Compound, _) if on_parent.is_list() && on_child.is_list() => {
                let mut refs = vec![RelationReference::CompoundChildId(on_parent)]
                    .clone_append(&mut common_child_references(on_parent));
                refs.push(RelationReference::NoRef);

                refs
            }
            (&Identifier::Compound, _) => vec![RelationReference::NoRef],
            (&Identifier::None, &RelationReference::NoRef) => common_child_references(on_parent),
            (&Identifier::None, _) if on_parent.is_list() && on_child.is_list() => {
                common_child_references(on_parent).clone_push(&RelationReference::NoRef)
            }
            (&Identifier::None, _) => vec![RelationReference::NoRef],
        }
    } else {
        match child_id {
            Identifier::Simple => vec![RelationReference::SimpleChildId(on_parent)],
            Identifier::Compound => vec![RelationReference::CompoundChildId(on_parent)],
            _ => vec![RelationReference::ChildReference(on_parent)],
        }
    }
}
