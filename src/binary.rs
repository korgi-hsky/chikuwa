pub mod instr;
pub mod ty;
pub mod value;

#[derive(Debug, PartialEq, Eq)]
pub struct Module {
    pub func_section: Option<FuncSection>,
    pub type_section: Option<TypeSection>,
    pub code_section: Option<CodeSection>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TypeSection(pub Vec<ty::Recursive>);

#[derive(Debug, PartialEq, Eq)]
pub struct FuncSection(pub Vec<u32>);

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
