use crate::field::*;
use prisma_value::{PrismaListValue, PrismaValue};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConditionValue {
    Value(PrismaValue),
    FieldRef(ScalarFieldRef),
}

impl ConditionValue {
    pub fn value(pv: PrismaValue) -> Self {
        Self::Value(pv)
    }

    pub fn reference(sf: ScalarFieldRef) -> Self {
        Self::FieldRef(sf)
    }

    pub fn into_value(self) -> Option<PrismaValue> {
        if let Self::Value(pv) = self {
            Some(pv)
        } else {
            None
        }
    }

    pub fn into_reference(self) -> Option<ScalarFieldRef> {
        if let Self::FieldRef(sf) = self {
            Some(sf)
        } else {
            None
        }
    }

    pub fn as_value(&self) -> Option<&PrismaValue> {
        if let Self::Value(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_field_ref(&self) -> Option<&ScalarFieldRef> {
        if let Self::FieldRef(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl From<PrismaValue> for ConditionValue {
    fn from(pv: PrismaValue) -> Self {
        Self::value(pv)
    }
}

impl From<ScalarFieldRef> for ConditionValue {
    fn from(sf: ScalarFieldRef) -> Self {
        Self::reference(sf)
    }
}

impl From<&ScalarFieldRef> for ConditionValue {
    fn from(sf: &ScalarFieldRef) -> Self {
        Self::reference(sf.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConditionListValue {
    List(PrismaListValue),
    FieldRef(ScalarFieldRef),
}

impl ConditionListValue {
    pub fn list<T>(vals: Vec<T>) -> Self
    where
        T: Into<PrismaValue>,
    {
        Self::List(vals.into_iter().map(Into::into).collect())
    }

    pub fn reference(sf: ScalarFieldRef) -> Self {
        Self::FieldRef(sf)
    }

    pub fn len(&self) -> usize {
        match self {
            ConditionListValue::List(list) => list.len(),
            ConditionListValue::FieldRef(_) => 1,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_field_ref(&self) -> Option<&ScalarFieldRef> {
        if let Self::FieldRef(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl From<PrismaListValue> for ConditionListValue {
    fn from(pv: PrismaListValue) -> Self {
        Self::list(pv)
    }
}

impl From<ScalarFieldRef> for ConditionListValue {
    fn from(sf: ScalarFieldRef) -> Self {
        Self::reference(sf)
    }
}
