pub mod instr;
pub mod ty;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Context {
    pub types: Vec<ty::Defined>,
    pub recs: Vec<ty::Sub>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Module {
    pub types: Vec<ty::Recursive>,
    pub funcs: Vec<Func>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Func {
    pub ty: ty::Func,
    pub locals: Vec<ty::Value>,
    pub expr: instr::Expression,
}
