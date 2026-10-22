#[derive(Debug, Clone)]
pub enum IR {
    ConstInt(i64),
    ConstFloat(f64),
    ConstBool(bool),
    LoadVar(String),
    StoreVar(String),
    LoadPtr(usize),
    StorePtr(usize, Box<IR>),
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Neg,
    CmpLt,
    CmpGt,
    CmpLe,
    CmpGe,
    CmpEq,
    CmpNeq,
    And,
    Or,
    Not,
    Shl,
    Shr,
    Band,
    Bor,
    Bxor,
    JmpIfFalse(usize),
    Jmp(usize),
    Label(usize),
    CallNative(String, usize),
    CallFFI(usize, usize),
    Ret,
    Drop,
    Dup,
    Swap,
    PeekI64,
    PokeI64,
    PeekByte,
    PokeByte,
    MakeClosure(usize),
    CallClosure,
}

#[derive(Debug, Clone)]
pub struct IRBlock {
    pub instructions: Vec<IR>,
}

impl IRBlock {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
        }
    }

    pub fn push(&mut self, inst: IR) {
        self.instructions.push(inst);
    }
}

impl Default for IRBlock {
    fn default() -> Self {
        Self::new()
    }
}
