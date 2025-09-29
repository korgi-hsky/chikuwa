pub mod instr;
pub mod ty;

#[derive(Debug, PartialEq, Eq)]
pub struct Module {
    pub types: Vec<ty::Recursive>,
    pub funcs: Vec<Func>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Index<T>(pub usize, std::marker::PhantomData<T>);
pub type TypeIndex = Index<ty::Recursive>;
pub type LocalIndex = Index<ty::Value>;

#[derive(Debug, PartialEq, Eq)]
pub struct Func {
    pub ty: ty::Func,
    pub locals: Vec<ty::Value>,
    pub expr: instr::Expression,
}
