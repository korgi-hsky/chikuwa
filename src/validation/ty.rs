#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recursive {
    inner: std::rc::Rc<RecursiveInner>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecursiveInner {
    types: Vec<Sub>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Defined {
    rec: Recursive,
    proj: usize,
}

#[derive(Clone, Debug)]
pub struct DefinedRef {
    rec: std::rc::Weak<RecursiveInner>,
    proj: usize,
}

impl DefinedRef {
    pub fn rec(&self) -> Recursive {
        Recursive {
            inner: self
                .rec
                .upgrade()
                .expect("`DefinedRef` should be used inside `Recursive`"),
        }
    }
}

impl PartialEq for DefinedRef {
    fn eq(&self, other: &Self) -> bool {
        self.proj == other.proj && self.rec() == other.rec()
    }
}
impl Eq for DefinedRef {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeUse {
    TypeIdx(usize),
    RecTypeIdx(usize),
    Def(DefinedRef),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sub {
    pub is_final: bool,
    pub supers: Vec<TypeUse>,
    pub body: Composite,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Composite {
    Func(Func),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Func {
    params: Vec<Value>,
    returns: Vec<Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Num(Number),
    Bottom,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Number {
    I32,
}
