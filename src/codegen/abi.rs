#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reg {
    RAX, RCX, RDX, RBX, RSP, RBP, RSI, RDI,
    R8, R9, R10, R11, R12, R13, R14, R15,
    XMM0, XMM1, XMM2, XMM3, XMM4, XMM5, XMM6, XMM7,
}

impl Reg {
    pub fn is_float(self) -> bool {
        matches!(self, Reg::XMM0 | Reg::XMM1 | Reg::XMM2 | Reg::XMM3 |
                   Reg::XMM4 | Reg::XMM5 | Reg::XMM6 | Reg::XMM7)
    }

    pub fn encoding(self) -> u8 {
        match self {
            Reg::RAX => 0, Reg::RCX => 1, Reg::RDX => 2, Reg::RBX => 3,
            Reg::RSP => 4, Reg::RBP => 5, Reg::RSI => 6, Reg::RDI => 7,
            Reg::R8 => 0, Reg::R9 => 1, Reg::R10 => 2, Reg::R11 => 3,
            Reg::R12 => 4, Reg::R13 => 5, Reg::R14 => 6, Reg::R15 => 7,
            Reg::XMM0 => 0, Reg::XMM1 => 1, Reg::XMM2 => 2, Reg::XMM3 => 3,
            Reg::XMM4 => 4, Reg::XMM5 => 5, Reg::XMM6 => 6, Reg::XMM7 => 7,
        }
    }

    pub fn needs_rex(self) -> bool {
        matches!(self, Reg::R8 | Reg::R9 | Reg::R10 | Reg::R11 | Reg::R12 |
                   Reg::R13 | Reg::R14 | Reg::R15)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ArgReg {
    pub reg: Reg,
    pub is_float: bool,
}

/// Windows x64 ABI: integer args in RCX, RDX, R8, R9; float args in XMM0-XMM3.
/// 32 bytes of shadow space required before CALL.
pub fn arg_registers(count: usize) -> Vec<ArgReg> {
    let mut regs = Vec::new();
    let mut int_idx = 0;
    let mut float_idx = 0;
    for _ in 0..count {
        if float_idx < 4 {
            regs.push(ArgReg {
                reg: match float_idx {
                    0 => Reg::XMM0,
                    1 => Reg::XMM1,
                    2 => Reg::XMM2,
                    _ => Reg::XMM3,
                },
                is_float: true,
            });
            float_idx += 1;
        } else if int_idx < 4 {
            regs.push(ArgReg {
                reg: match int_idx {
                    0 => Reg::RCX,
                    1 => Reg::RDX,
                    2 => Reg::R8,
                    _ => Reg::R9,
                },
                is_float: false,
            });
            int_idx += 1;
        } else {
            // Stack-passed arguments (not yet implemented)
            regs.push(ArgReg {
                reg: Reg::RSP,
                is_float: false,
            });
        }
    }
    regs
}

pub const SHADOW_SPACE: u32 = 32;

pub fn ret_reg(is_float: bool) -> Reg {
    if is_float {
        Reg::XMM0
    } else {
        Reg::RAX
    }
}
