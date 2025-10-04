#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recursive {
    inner: Box<RecursiveInner>,
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
            inner: RecursiveInner { types: value }.into(),
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

    fn close(&self, cx: &super::Context) -> Self {
        Self::new(self.iter().map(|s| s.close(cx)).collect())
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

    pub fn close(&self, cx: &super::Context) -> Self {
        Self {
            rec: self.rec.close(cx),
            proj: self.proj,
        }
    }
}

trait Substitute: Sized {
    fn substitute(&self, f: &impl Fn(&TypeUse) -> TypeUse) -> Self;

    fn rollup(&self, start: usize, end: usize) -> Self {
        self.substitute(&|u| match u {
            TypeUse::TypeIdx(i) if (start..end).contains(i) => TypeUse::RecTypeIdx(i - start),
            other => other.clone(),
        })
    }

    fn unroll(&self, rec: &Recursive) -> Self {
        self.substitute(&|u| match u {
            TypeUse::RecTypeIdx(i) => TypeUse::Def(Defined::new(rec, *i)),
            other => other.clone(),
        })
    }

    fn close(&self, cx: &super::Context) -> Self {
        self.substitute(&|u| match u {
            // Circular typeidx references are eliminated by rolling up recursive types
            TypeUse::TypeIdx(i) => TypeUse::Def(cx.types[*i].close(cx)),
            other => other.clone(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeUse {
    TypeIdx(usize),
    RecTypeIdx(usize),
    Def(Defined),
}

impl From<u32> for TypeUse {
    fn from(value: u32) -> Self {
        Self::TypeIdx(value as usize)
    }
}

impl Substitute for TypeUse {
    fn substitute(&self, f: &impl Fn(&TypeUse) -> TypeUse) -> Self {
        f(self)
    }
}

impl TypeUse {
    fn validate(&self, cx: &mut super::Context) -> anyhow::Result<()> {
        match self {
            Self::TypeIdx(i) => {
                anyhow::ensure!(*i < cx.types.len(), "type {i} is not defined");
            }
            Self::RecTypeIdx(i) => {
                anyhow::ensure!(*i < cx.recs.len(), "recursive type {i} is not defined");
            }
            Self::Def(def) => {
                let Defined { rec, proj } = def;
                rec.validate(cx)?;
                anyhow::ensure!(*proj < rec.len(), "sub type {proj} is not defined");
            }
        }
        Ok(())
    }

    /// [`self`] should be already validated.
    fn get_type(&self, cx: &mut super::Context) -> Sub {
        match self {
            Self::TypeIdx(i) => cx.types[*i].unroll(),
            Self::RecTypeIdx(i) => cx.recs[*i].clone(),
            Self::Def(def) => def.unroll(),
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

impl Substitute for Sub {
    fn substitute(&self, f: &impl Fn(&TypeUse) -> TypeUse) -> Self {
        Self {
            is_final: self.is_final,
            supers: self.supers.iter().map(|u| u.substitute(f)).collect(),
            body: self.body.substitute(f),
        }
    }
}

impl Sub {
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

impl Substitute for Composite {
    fn substitute(&self, f: &impl Fn(&TypeUse) -> TypeUse) -> Self {
        match self {
            Self::Func(func) => Self::Func(func.substitute(f)),
        }
    }
}

impl Composite {
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

impl Substitute for Func {
    fn substitute(&self, f: &impl Fn(&TypeUse) -> TypeUse) -> Self {
        Self {
            params: self.params.iter().map(|v| v.substitute(f)).collect(),
            returns: self.returns.iter().map(|v| v.substitute(f)).collect(),
        }
    }
}

impl Func {
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

impl Substitute for Value {
    fn substitute(&self, f: &impl Fn(&TypeUse) -> TypeUse) -> Self {
        match self {
            Self::Num(n) => Self::Num(n.substitute(f)),
            Self::Bottom => Self::Bottom,
        }
    }
}

impl Value {
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

impl Substitute for Number {
    fn substitute(&self, _f: &impl Fn(&TypeUse) -> TypeUse) -> Self {
        self.clone()
    }
}
