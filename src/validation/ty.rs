#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Defined {
    rec: Recursive,
    proj: usize,
}

impl Defined {
    pub fn new(rec: Recursive, proj: usize) -> Self {
        Self { rec, proj }
    }

    pub fn rollup(rec: &Recursive, start_typeidx: usize) -> Vec<Self> {
        let rec = rec.rollup(start_typeidx, start_typeidx + rec.len());
        (0..rec.len())
            .map(|proj| Self::new(rec.clone(), proj))
            .collect()
    }

    pub fn unroll(&self) -> Sub {
        self.rec.unroll(&self.rec)[self.proj].clone()
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
            TypeUse::RecTypeIdx(i) => TypeUse::Def(Defined::new(rec.clone(), *i)),
            other => other.clone(),
        })
    }

    fn close(&self, cx: &super::Context) -> Self {
        self.substitute(&|u| match u {
            TypeUse::TypeIdx(i) => TypeUse::Def(cx.types[*i].close(cx)),
            other => other.clone(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recursive {
    pub types: Vec<Sub>,
}

impl std::ops::Deref for Recursive {
    type Target = Vec<Sub>;
    fn deref(&self) -> &Self::Target {
        &self.types
    }
}

impl From<crate::binary::ty::Recursive> for Recursive {
    fn from(value: crate::binary::ty::Recursive) -> Self {
        Self::new(value.0.into_iter().map(Into::into).collect())
    }
}

impl Substitute for Recursive {
    fn substitute(&self, f: &impl Fn(&TypeUse) -> TypeUse) -> Self {
        Self::new(self.iter().map(|s| s.substitute(f)).collect())
    }
}

impl Recursive {
    fn new(types: Vec<Sub>) -> Self {
        Self { types }
    }

    pub fn validate(&self, cx: &mut super::Context) -> anyhow::Result<()> {
        cx.recs = self.types.clone();
        self.iter().try_for_each(|s| s.validate(cx))?;
        Ok(())
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

    /// Ensure that [`self`] is valid.
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
            self.body.should_be(&sup.body)?;
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

    fn should_be(&self, other: &Self) -> anyhow::Result<()> {
        match (self, other) {
            (Self::Func(a), Self::Func(b)) => a.should_be(b),
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

    fn should_be(&self, other: &Self) -> anyhow::Result<()> {
        if self.params.len() != other.params.len() {
            return subtyping_err(&self.params, &other.params);
        }
        if self.returns.len() != other.returns.len() {
            return subtyping_err(&self.returns, &other.returns);
        }
        // other.params <: self.params
        // self.returns <: other.returns
        // -----------------------------
        //         self <: other
        other
            .params
            .iter()
            .zip(&self.params)
            .try_for_each(|(a, b)| a.should_be(b))?;
        self.returns
            .iter()
            .zip(&other.returns)
            .try_for_each(|(a, b)| a.should_be(b))?;
        Ok(())
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

    fn should_be(&self, other: &Self) -> anyhow::Result<()> {
        match (self, other) {
            (Self::Bottom, _) => Ok(()),
            (Self::Num(a), Self::Num(b)) => a.should_be(b),
            _ => subtyping_err(self, other),
        }
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

impl Number {
    fn should_be(&self, other: &Self) -> anyhow::Result<()> {
        match (self, other) {
            (Self::I32, Self::I32) => Ok(()),
        }
    }
}

fn subtyping_err<T: std::fmt::Debug>(a: &T, b: &T) -> anyhow::Result<()> {
    anyhow::bail!("{a:?} is not a subtype of {b:?}");
}
