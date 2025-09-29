pub mod instr;
pub mod ty;
pub mod value;

#[derive(Debug, PartialEq, Eq)]
pub struct Module {
    pub func_section: FuncSection,
    pub type_section: TypeSection,
    pub code_section: CodeSection,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Index<T>(pub u32, std::marker::PhantomData<T>);
pub type TypeIndex = Index<ty::Recursive>;
pub type LocalIndex = Index<Local>;

#[derive(Debug, PartialEq, Eq)]
pub struct TypeSection(pub Vec<ty::Recursive>);

#[derive(Debug, PartialEq, Eq)]
pub struct FuncSection(pub Vec<TypeIndex>);

#[derive(Debug, PartialEq, Eq)]
pub struct CodeSection(pub Vec<Func>);

#[derive(Debug, PartialEq, Eq)]
pub struct Func {
    pub locals: Vec<Local>,
    pub expr: instr::Expression,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Local {
    pub num: u32,
    pub ty: ty::Value,
}
