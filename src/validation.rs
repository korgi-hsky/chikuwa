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

impl TryFrom<crate::binary::Module> for Module {
    type Error = anyhow::Error;

    fn try_from(value: crate::binary::Module) -> Result<Self, Self::Error> {
        let mut module = Module::default();
        let mut cx = Context::default();

        for raw in value.type_section.map_or_else(Vec::new, |s| s.0) {
            let rec = ty::Recursive::from(raw);
            cx.types.extend(rec.rollup(cx.types.len()));
            rec.validate(&mut cx)?;
            module.types.push(rec);
        }

        Ok(module)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Func {
    pub ty: ty::Func,
    pub locals: Vec<ty::Value>,
    pub expr: instr::Expression,
}
