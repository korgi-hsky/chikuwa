#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recursive {
    inner: std::rc::Rc<RecursiveInner>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecursiveInner {
    types: Vec<Sub>,
}

impl std::ops::Deref for Recursive {
    type Target = Vec<Sub>;
    fn deref(&self) -> &Self::Target {
        &self.inner.deref().types
    }
}

impl From<crate::binary::ty::Recursive> for Recursive {
    fn from(value: crate::binary::ty::Recursive) -> Self {
        Self::new(value.0.into_iter().map(Into::into).collect())
    }
}

impl Recursive {
    fn new(value: Vec<Sub>) -> Self {
        Self {
            inner: std::rc::Rc::new(RecursiveInner { types: value }),
        }
    }

    pub fn rollup(&self, start_typeidx: usize) -> Vec<Defined> {
        let start = start_typeidx;
        let end = start + self.len();
        let rec = Self::new(self.iter().map(|s| s.rollup(start, end)).collect());
        (0..self.len())
            .map(|proj| Defined::new(&rec, proj))
            .collect()
    }

    fn unroll(&self) -> Self {
        Self::new(self.iter().map(|s| s.unroll(self)).collect())
    }

    pub fn validate(&self, cx: &mut super::Context) -> anyhow::Result<()> {
        cx.recs = self.inner.types.clone();
        self.iter().try_for_each(|s| s.validate(cx))?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Defined {
    rec: Recursive,
    proj: usize,
}

impl Defined {
    pub fn new(rec: &Recursive, proj: usize) -> Self {
        Self {
            rec: rec.clone(),
            proj,
        }
    }

    pub fn unroll(&self) -> Sub {
        self.rec.unroll()[self.proj].clone()
    }
}

#[derive(Clone, Debug)]
pub struct DefinedRef {
    rec: std::rc::Weak<RecursiveInner>,
    proj: usize,
}

impl DefinedRef {
    fn new(rec: &Recursive, proj: usize) -> Self {
        Self {
            rec: std::rc::Rc::downgrade(&rec.inner),
            proj,
        }
    }
}

impl DefinedRef {
    pub fn get(&self) -> Defined {
        Defined {
            rec: Recursive {
                inner: self
                    .rec
                    .upgrade()
                    .expect("`DefinedRef` should be used inside `Recursive`"),
            },
            proj: self.proj,
        }
    }
}

impl PartialEq for DefinedRef {
    fn eq(&self, other: &Self) -> bool {
        self.proj == other.proj && self.get() == other.get()
    }
}
impl Eq for DefinedRef {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeUse {
    TypeIdx(usize),
    RecTypeIdx(usize),
    Def(DefinedRef),
}

impl From<u32> for TypeUse {
    fn from(value: u32) -> Self {
        Self::TypeIdx(value as usize)
    }
}

impl TypeUse {
    fn rollup(&self, start: usize, end: usize) -> Self {
        match self {
            Self::TypeIdx(i) if (start..end).contains(i) => Self::RecTypeIdx(i - start),
            other => other.clone(),
        }
    }

    fn unroll(&self, rec: &Recursive) -> Self {
        match self {
            Self::RecTypeIdx(i) => Self::Def(DefinedRef::new(rec, *i)),
            other => other.clone(),
        }
    }

    fn validate(&self, cx: &mut super::Context) -> anyhow::Result<()> {
        match self {
            Self::TypeIdx(i) => {
                anyhow::ensure!(*i < cx.types.len(), "type {i} is not defined");
            }
            Self::RecTypeIdx(i) => {
                anyhow::ensure!(*i < cx.recs.len(), "recursive type {i} is not defined");
            }
            Self::Def(def) => {
                let Defined { rec, proj } = def.get();
                rec.validate(cx)?;
                anyhow::ensure!(proj < rec.len(), "sub type {proj} is not defined");
            }
        }
        Ok(())
    }

    /// [`self`] should be already validated.
    fn get_type(&self, cx: &mut super::Context) -> Sub {
        match self {
            Self::TypeIdx(i) => cx.types[*i].unroll(),
            Self::RecTypeIdx(i) => cx.recs[*i].clone(),
            Self::Def(def) => def.get().unroll(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sub {
    pub is_final: bool,
    pub supers: Vec<TypeUse>,
    pub body: Composite,
}

impl From<crate::binary::ty::Sub> for Sub {
    fn from(value: crate::binary::ty::Sub) -> Self {
        Self {
            is_final: value.is_final,
            supers: value.supers.into_iter().map(Into::into).collect(),
            body: value.ty.into(),
        }
    }
}

impl Sub {
    fn rollup(&self, start: usize, end: usize) -> Self {
        Self {
            is_final: self.is_final,
            supers: self.supers.iter().map(|u| u.rollup(start, end)).collect(),
            body: self.body.rollup(start, end),
        }
    }

    fn unroll(&self, rec: &Recursive) -> Self {
        Self {
            is_final: self.is_final,
            supers: self.supers.iter().map(|u| u.unroll(rec)).collect(),
            body: self.body.unroll(rec),
        }
    }

    fn validate(&self, cx: &mut super::Context) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.supers.len() <= 1,
            "more than one supertype is not allowed",
        );
        for u in &self.supers {
            u.validate(cx)?;
            let sup = u.get_type(cx);
            anyhow::ensure!(!sup.is_final, "cannot specify final type as supertype");
            // TODO: check subtyping `self <: sup`
        }
        self.body.validate()?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Composite {
    Func(Func),
}

impl From<crate::binary::ty::Composite> for Composite {
    fn from(value: crate::binary::ty::Composite) -> Self {
        match value {
            crate::binary::ty::Composite::Func { params, returns } => Self::Func(Func {
                params: params.into_iter().map(Into::into).collect(),
                returns: returns.into_iter().map(Into::into).collect(),
            }),
        }
    }
}

impl Composite {
    fn rollup(&self, start: usize, end: usize) -> Self {
        match self {
            Self::Func(f) => Self::Func(f.rollup(start, end)),
        }
    }

    fn unroll(&self, rec: &Recursive) -> Self {
        match self {
            Self::Func(f) => Self::Func(f.unroll(rec)),
        }
    }

    fn validate(&self) -> anyhow::Result<()> {
        match self {
            Self::Func(f) => f.validate(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Func {
    params: Vec<Value>,
    returns: Vec<Value>,
}

impl Func {
    fn rollup(&self, start: usize, end: usize) -> Self {
        Self {
            params: self.params.iter().map(|v| v.rollup(start, end)).collect(),
            returns: self.returns.iter().map(|v| v.rollup(start, end)).collect(),
        }
    }

    fn unroll(&self, rec: &Recursive) -> Self {
        Self {
            params: self.params.iter().map(|v| v.unroll(rec)).collect(),
            returns: self.returns.iter().map(|v| v.unroll(rec)).collect(),
        }
    }

    fn validate(&self) -> anyhow::Result<()> {
        self.params
            .iter()
            .chain(&self.returns)
            .try_for_each(|v| v.validate())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Num(Number),
    Bottom,
}

impl From<crate::binary::ty::Value> for Value {
    fn from(value: crate::binary::ty::Value) -> Self {
        match value {
            crate::binary::ty::Value::Num(n) => Self::Num(n.into()),
        }
    }
}

impl Value {
    fn rollup(&self, start: usize, end: usize) -> Self {
        _ = (start, end);
        self.clone()
    }

    fn unroll(&self, rec: &Recursive) -> Self {
        _ = rec;
        self.clone()
    }

    fn validate(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Number {
    I32,
}

impl From<crate::binary::ty::Number> for Number {
    fn from(value: crate::binary::ty::Number) -> Self {
        match value {
            crate::binary::ty::Number::I32 => Self::I32,
        }
    }
}
