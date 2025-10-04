#[derive(Debug, PartialEq, Eq)]
pub enum Instruction {
    // parametric
    Nop,

    // variable
    LocalGet(u32),

    // numeric
    I32Add,

    End,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Expression(pub Vec<Instruction>);
