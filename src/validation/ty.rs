#[derive(Debug, PartialEq, Eq)]
pub struct Recursive(pub Vec<Sub>);

#[derive(Debug, PartialEq, Eq)]
pub struct Sub {
    pub is_final: bool,
    pub supers: Vec<TypeUse>,
    pub ty: Composite,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Composite {
    Func(Func),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Func {
    params: Vec<Value>,
    returns: Vec<Value>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TypeUse {
    Index(super::TypeIndex),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Value {
    Num(Number),
    Bottom,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Number {
    I32,
}
