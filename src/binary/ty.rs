#[derive(Debug, PartialEq, Eq)]
pub struct Recursive(pub Vec<Sub>);

#[derive(Debug, PartialEq, Eq)]
pub struct Sub {
    pub is_final: bool,
    pub supers: Vec<super::TypeIndex>,
    pub ty: Composite,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Composite {
    Fn {
        params: Vec<Value>,
        returns: Vec<Value>,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Value {
    Num(Number),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Number {
    I32,
}
