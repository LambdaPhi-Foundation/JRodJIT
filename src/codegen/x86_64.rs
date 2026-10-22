use crate::codegen::abi::*;
use crate::ir::IR;
use crate::mem::align_to;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpSize {
    None,
    Byte,
    Word,
    DWord,
    QWord,
}

struct REX {
    w: bool,
    r: bool,
    x: bool,
    b: bool,
}

impl REX {
    fn new(w: bool, r: bool, x: bool, b: bool) -> Self {
        Self { w, r, x, b }
    }

    fn byte(self) -> u8 {
        0x40
            | if self.w { 8 } else { 0 }
            | if self.r { 4 } else { 0 }
            | if self.x { 2 } else { 0 }
            | if self.b { 1 } else { 0 }
    }
}

pub struct X86_64Emitter {
    buf: crate::mem::ExecBuffer,
    var_offsets: std::collections::HashMap<String, usize>,
    label_offsets: Vec<usize>,
    fixups: Vec<(usize, usize)>,
}

impl X86_64Emitter {
    pub fn new(initial_size: usize, var_offsets: std::collections::HashMap<String, usize>) -> Result<Self, String> {
        Ok(Self {
            buf: crate::mem::ExecBuffer::new(initial_size)?,
            var_offsets,
            label_offsets: Vec::new(),
            fixups: Vec::new(),
        })
    }

    fn emit(&mut self, byte: u8) {
        self.buf.write(byte);
    }

    fn emit_rex(&mut self, w: bool, r: bool, x: bool, b: bool) {
        let rex = REX::new(w, r, x, b);
        let b_val = rex.byte();
        if b_val != 0x40 {
            self.emit(b_val);
        }
    }

    fn emit_modrm(&mut self, mod_val: u8, reg: u8, rm: u8) {
        self.emit((mod_val << 6) | (reg << 3) | rm);
    }

    fn emit_sib(&mut self, scale: u8, index: u8, base: u8) {
        self.emit((scale << 6) | (index << 3) | base);
    }

    fn emit_i8(&mut self, val: i8) {
        self.emit(val as u8);
    }

    fn emit_i32(&mut self, val: i32) {
        self.buf.write_i32_le(val);
    }

    fn emit_u64(&mut self, val: u64) {
        self.buf.write_u64_le(val);
    }

    pub fn add_label(&mut self) -> usize {
        let idx = self.label_offsets.len();
        self.label_offsets.push(self.buf.current_offset());
        idx
    }

    pub fn bind_label(&mut self, idx: usize) {
        if idx >= self.label_offsets.len() {
            self.label_offsets.resize(idx + 1, 0);
        }
        self.label_offsets[idx] = self.buf.current_offset();
    }

    pub fn mark_fixup(&mut self, label_idx: usize) {
        self.fixups.push((self.buf.current_offset(), label_idx));
    }

    fn resolve_fixups(&mut self) {
        for (offset, label_idx) in self.fixups.drain(..) {
            let target = self.label_offsets[label_idx];
            self.buf.patch_rel32(offset, target);
        }
    }

    fn mov_imm64_to_reg(&mut self, reg: Reg, imm: u64) {
        self.emit_rex(true, false, false, reg.needs_rex());
        self.emit(0xB8 | reg.encoding());
        self.emit_u64(imm);
    }

    fn mov_reg_to_reg(&mut self, dst: Reg, src: Reg) {
        self.emit_rex(true, src.needs_rex(), false, dst.needs_rex());
        self.emit(0x89);
        self.emit_modrm(3, src.encoding(), dst.encoding());
    }

    fn mov_reg_to_mem(&mut self, base: Reg, disp: i32, src: Reg) {
        self.emit_rex(true, src.needs_rex(), false, base.needs_rex());
        self.emit(0x89);
        if disp == 0 && base != Reg::RBP && base != Reg::R13 {
            self.emit_modrm(0, src.encoding(), base.encoding());
        } else if (disp as i8 as i32) == disp {
            self.emit_modrm(1, src.encoding(), base.encoding());
            self.emit_i8(disp as i8);
        } else {
            self.emit_modrm(2, src.encoding(), base.encoding());
            self.emit_i32(disp);
        }
    }

    fn mov_mem_to_reg(&mut self, dst: Reg, base: Reg, disp: i32) {
        self.emit_rex(true, dst.needs_rex(), false, base.needs_rex());
        self.emit(0x8B);
        if disp == 0 && base != Reg::RBP && base != Reg::R13 {
            self.emit_modrm(0, dst.encoding(), base.encoding());
        } else if (disp as i8 as i32) == disp {
            self.emit_modrm(1, dst.encoding(), base.encoding());
            self.emit_i8(disp as i8);
        } else {
            self.emit_modrm(2, dst.encoding(), base.encoding());
            self.emit_i32(disp);
        }
    }

    fn add_reg_to_reg(&mut self, dst: Reg, src: Reg) {
        self.emit_rex(true, src.needs_rex(), false, dst.needs_rex());
        self.emit(0x01);
        self.emit_modrm(3, src.encoding(), dst.encoding());
    }

    fn sub_reg_from_reg(&mut self, dst: Reg, src: Reg) {
        self.emit_rex(true, src.needs_rex(), false, dst.needs_rex());
        self.emit(0x29);
        self.emit_modrm(3, src.encoding(), dst.encoding());
    }

    fn imul_reg_reg(&mut self, dst: Reg, src: Reg) {
        self.emit_rex(true, src.needs_rex(), false, dst.needs_rex());
        self.emit(0x0F);
        self.emit(0xAF);
        self.emit_modrm(3, src.encoding(), dst.encoding());
    }

    fn xor_reg_reg(&mut self, dst: Reg, src: Reg) {
        self.emit_rex(true, src.needs_rex(), false, dst.needs_rex());
        self.emit(0x31);
        self.emit_modrm(3, src.encoding(), dst.encoding());
    }

    fn cmp_reg_reg(&mut self, a: Reg, b: Reg) {
        self.emit_rex(true, b.needs_rex(), false, a.needs_rex());
        self.emit(0x3B);
        self.emit_modrm(3, b.encoding(), a.encoding());
    }

    fn movsd_reg_reg(&mut self, dst: Reg, src: Reg) {
        self.emit(0xF2);
        self.emit(0x0F);
        self.emit(0x11);
        self.emit_modrm(3, src.encoding(), dst.encoding());
    }

    fn addsd_reg_reg(&mut self, dst: Reg, src: Reg) {
        self.emit(0xF2);
        self.emit(0x0F);
        self.emit(0x58);
        self.emit_modrm(3, src.encoding(), dst.encoding());
    }

    fn subsd_reg_reg(&mut self, dst: Reg, src: Reg) {
        self.emit(0xF2);
        self.emit(0x0F);
        self.emit(0x5C);
        self.emit_modrm(3, src.encoding(), dst.encoding());
    }

    fn mulsd_reg_reg(&mut self, dst: Reg, src: Reg) {
        self.emit(0xF2);
        self.emit(0x0F);
        self.emit(0x59);
        self.emit_modrm(3, src.encoding(), dst.encoding());
    }

    fn divsd_reg_reg(&mut self, dst: Reg, src: Reg) {
        self.emit(0xF2);
        self.emit(0x0F);
        self.emit(0x5E);
        self.emit_modrm(3, src.encoding(), dst.encoding());
    }

    fn comisd_reg_reg(&mut self, a: Reg, b: Reg) {
        self.emit(0x66);
        self.emit(0x0F);
        self.emit(0x2F);
        self.emit_modrm(3, b.encoding(), a.encoding());
    }

    fn jmp_rel32(&mut self, fixup_offset: usize) {
        self.emit(0xE9);
        self.mark_fixup(fixup_offset);
        self.emit_i32(0); // placeholder
    }

    fn jcc_rel32(&mut self, opcode: u8, fixup_offset: usize) {
        self.emit(0x0F);
        self.emit(opcode);
        self.mark_fixup(fixup_offset);
        self.emit_i32(0); // placeholder
    }

    fn call_abs_reg(&mut self, reg: Reg) {
        self.emit_rex(true, false, false, reg.needs_rex());
        self.emit(0xFF);
        self.emit_modrm(3, 2, reg.encoding());
    }

    fn sub_imm_from_rsp(&mut self, imm: u32) {
        self.emit(0x48);
        self.emit(0x81);
        self.emit(0xEC);
        self.emit_i32(imm as i32);
    }

    fn add_imm_to_rsp(&mut self, imm: u32) {
        self.emit(0x48);
        self.emit(0x81);
        self.emit(0xC4);
        self.emit_i32(imm as i32);
    }

    fn mov_imm32_to_reg(&mut self, reg: Reg, imm: i32) {
        self.emit(0xB8 | reg.encoding());
        self.emit_i32(imm);
    }

    pub fn compile_block(&mut self, block: &[IR], stack_size: usize) -> Result<*mut u8, String> {
        // --- Register pool ---
        // Only caller-saved registers (Windows x64 ABI):
        // Integer: RCX, R8, R9, R10, R11 (RAX reserved for return/div, RDX reserved for div)
        // Float:   XMM0-XMM5 (XMM6-XMM15 are callee-saved)
        // RBP = frame pointer, RSP = stack pointer (never used as general purpose).
        let mut free_int: Vec<Reg> = vec![
            Reg::RCX, Reg::R8, Reg::R9, Reg::R10, Reg::R11,
        ];
        let mut free_float: Vec<Reg> = vec![
            Reg::XMM0, Reg::XMM1, Reg::XMM2, Reg::XMM3,
            Reg::XMM4, Reg::XMM5,
        ];
        let mut vals: Vec<(Reg, bool)> = Vec::new(); // stack of (register, is_float)

        // --- Prologue ---
        self.emit(0x55); // push rbp
        self.emit(0x48);
        self.emit(0x89);
        self.emit(0xE5); // mov rbp, rsp
        let aligned = align_to(stack_size, 16);
        if aligned > 0 {
            self.sub_imm_from_rsp(aligned as u32);
        }

        for inst in block {
            match inst {
                IR::ConstInt(v) => {
                    let reg = free_int.pop().ok_or("Out of integer registers".to_string())?;
                    self.mov_imm64_to_reg(reg, *v as u64);
                    vals.push((reg, false));
                }
                IR::ConstFloat(_v) => {
                    let reg = free_float.pop().ok_or("Out of float registers".to_string())?;
                    // Zero the XMM register (contains 0.0)
                    self.emit(0x66);
                    self.emit(0x0F);
                    self.emit(0xEF);
                    self.emit_modrm(3, reg.encoding(), reg.encoding()); // pxor xmm, xmm
                    vals.push((reg, true));
                }
                IR::ConstBool(v) => {
                    let reg = free_int.pop().ok_or("Out of integer registers".to_string())?;
                    self.mov_imm32_to_reg(reg, if *v { 1 } else { 0 });
                    vals.push((reg, false));
                }
                IR::LoadVar(name) => {
                    let offset = *self.var_offsets.get(name).unwrap_or(&0) as i32;
                    let reg = free_int.pop().ok_or("Out of integer registers".to_string())?;
                    self.mov_mem_to_reg(reg, Reg::RBP, -offset);
                    vals.push((reg, false));
                }
                IR::StoreVar(name) => {
                    let (val_reg, _) = vals.pop().unwrap();
                    let offset = *self.var_offsets.get(name).unwrap_or(&0) as i32;
                    self.mov_reg_to_mem(Reg::RBP, -offset, val_reg);
                    free_int.push(val_reg);
                }
                IR::LoadVar(name) => {
                    let offset = *self.var_offsets.get(name).unwrap_or(&0) as i32;
                    let reg = free_int.pop().ok_or("Out of integer registers".to_string())?;
                    self.mov_mem_to_reg(reg, Reg::RBP, -offset);
                    vals.push((reg, false));
                }
                IR::Add => {
                    let (b_reg, b_float) = vals.pop().unwrap();
                    let (a_reg, a_float) = vals.pop().unwrap();
                    if a_float && b_float {
                        self.addsd_reg_reg(a_reg, b_reg);
                        free_float.push(b_reg);
                        vals.push((a_reg, true));
                    } else {
                        self.add_reg_to_reg(a_reg, b_reg);
                        free_int.push(b_reg);
                        vals.push((a_reg, false));
                    }
                }
                IR::Sub => {
                    let (b_reg, b_float) = vals.pop().unwrap();
                    let (a_reg, a_float) = vals.pop().unwrap();
                    if a_float && b_float {
                        self.subsd_reg_reg(a_reg, b_reg);
                        free_float.push(b_reg);
                        vals.push((a_reg, true));
                    } else {
                        self.sub_reg_from_reg(a_reg, b_reg);
                        free_int.push(b_reg);
                        vals.push((a_reg, false));
                    }
                }
                IR::Mul => {
                    let (b_reg, b_float) = vals.pop().unwrap();
                    let (a_reg, a_float) = vals.pop().unwrap();
                    if a_float && b_float {
                        self.mulsd_reg_reg(a_reg, b_reg);
                        free_float.push(b_reg);
                        vals.push((a_reg, true));
                    } else {
                        self.imul_reg_reg(a_reg, b_reg);
                        free_int.push(b_reg);
                        vals.push((a_reg, false));
                    }
                }
                IR::Div => {
                    let (b_reg, b_float) = vals.pop().unwrap();
                    let (a_reg, a_float) = vals.pop().unwrap();
                    if a_float && b_float {
                        self.divsd_reg_reg(a_reg, b_reg);
                        free_float.push(b_reg);
                        vals.push((a_reg, true));
                    } else {
                        self.mov_reg_to_reg(Reg::RAX, a_reg);
                        if a_reg != Reg::RAX {
                            free_int.push(a_reg);
                        }
                        self.emit(0x48);
                        self.emit(0x99); // cqo
                        self.emit_rex(true, false, false, b_reg.needs_rex());
                        self.emit(0x48 | if b_reg.needs_rex() { 1 } else { 0 });
                        self.emit(0xF7);
                        self.emit_modrm(3, 7, b_reg.encoding());
                        free_int.push(b_reg);
                        vals.push((Reg::RAX, false));
                    }
                }
                IR::Rem => {
                    let (b_reg, b_float) = vals.pop().unwrap();
                    let (a_reg, a_float) = vals.pop().unwrap();
                    if a_float && b_float {
                        free_float.push(b_reg);
                        free_float.push(a_reg);
                        vals.push((Reg::XMM0, true));
                    } else {
                        self.mov_reg_to_reg(Reg::RAX, a_reg);
                        if a_reg != Reg::RAX {
                            free_int.push(a_reg);
                        }
                        self.emit(0x48);
                        self.emit(0x99); // cqo
                        self.emit_rex(true, false, false, b_reg.needs_rex());
                        self.emit(0x48 | if b_reg.needs_rex() { 1 } else { 0 });
                        self.emit(0xF7);
                        self.emit_modrm(3, 6, b_reg.encoding()); // idiv, remainder in RDX
                        free_int.push(b_reg);
                        self.mov_reg_to_reg(Reg::RAX, Reg::RDX); // move remainder to RAX
                        free_int.push(Reg::RDX);
                        vals.push((Reg::RAX, false));
                    }
                }
                IR::Neg => {
                    let (reg, is_float) = vals.pop().unwrap();
                    if is_float {
                        let zero_reg = free_float.pop().ok_or("Out of float registers".to_string())?;
                        self.emit(0x66);
                        self.emit(0x0F);
                        self.emit(0xEF);
                        self.emit_modrm(3, zero_reg.encoding(), zero_reg.encoding()); // pxor zero, zero
                        self.subsd_reg_reg(zero_reg, reg);
                        free_float.push(reg);
                        vals.push((zero_reg, true));
                    } else {
                        self.emit_rex(true, false, false, reg.needs_rex());
                        self.emit(0x48 | if reg.needs_rex() { 1 } else { 0 });
                        self.emit(0xF7);
                        self.emit_modrm(3, 3, reg.encoding()); // neg r/m64
                        vals.push((reg, false));
                    }
                }
                IR::CmpLt => {
                    let (b_reg, b_float) = vals.pop().unwrap();
                    let (a_reg, a_float) = vals.pop().unwrap();
                    if a_float && b_float {
                        self.comisd_reg_reg(a_reg, b_reg);
                        free_float.push(b_reg);
                        free_float.push(a_reg);
                        let res = free_int.pop().ok_or("Out of integer registers".to_string())?;
                        self.emit(0x0F);
                        self.emit(0x96); // setb
                        self.emit_modrm(3, 0, res.encoding());
                        self.emit(0x0F);
                        self.emit(0xB6); // movzx
                        self.emit_modrm(3, res.encoding(), res.encoding());
                        vals.push((res, false));
                    } else {
                        self.cmp_reg_reg(a_reg, b_reg);
                        free_int.push(b_reg);
                        free_int.push(a_reg);
                        let res = free_int.pop().ok_or("Out of integer registers".to_string())?;
                        self.emit(0x0F);
                        self.emit(0x9C); // setl
                        self.emit_modrm(3, 0, res.encoding());
                        self.emit(0x0F);
                        self.emit(0xB6); // movzx
                        self.emit_modrm(3, res.encoding(), res.encoding());
                        vals.push((res, false));
                    }
                }
                IR::CmpEq => {
                    let (b_reg, b_float) = vals.pop().unwrap();
                    let (a_reg, a_float) = vals.pop().unwrap();
                    if a_float && b_float {
                        self.comisd_reg_reg(a_reg, b_reg);
                        free_float.push(b_reg);
                        free_float.push(a_reg);
                        let res = free_int.pop().ok_or("Out of integer registers".to_string())?;
                        self.emit(0x0F);
                        self.emit(0x94); // setz
                        self.emit_modrm(3, 0, res.encoding());
                        self.emit(0x0F);
                        self.emit(0xB6); // movzx
                        self.emit_modrm(3, res.encoding(), res.encoding());
                        vals.push((res, false));
                    } else {
                        self.cmp_reg_reg(a_reg, b_reg);
                        free_int.push(b_reg);
                        free_int.push(a_reg);
                        let res = free_int.pop().ok_or("Out of integer registers".to_string())?;
                        self.emit(0x0F);
                        self.emit(0x94); // setz
                        self.emit_modrm(3, 0, res.encoding());
                        self.emit(0x0F);
                        self.emit(0xB6); // movzx
                        self.emit_modrm(3, res.encoding(), res.encoding());
                        vals.push((res, false));
                    }
                }
                IR::CmpNeq => {
                    let (b_reg, b_float) = vals.pop().unwrap();
                    let (a_reg, a_float) = vals.pop().unwrap();
                    if a_float && b_float {
                        self.comisd_reg_reg(a_reg, b_reg);
                        free_float.push(b_reg);
                        free_float.push(a_reg);
                        let res = free_int.pop().ok_or("Out of integer registers".to_string())?;
                        self.emit(0x0F);
                        self.emit(0x95); // setnz
                        self.emit_modrm(3, 0, res.encoding());
                        self.emit(0x0F);
                        self.emit(0xB6); // movzx
                        self.emit_modrm(3, res.encoding(), res.encoding());
                        vals.push((res, false));
                    } else {
                        self.cmp_reg_reg(a_reg, b_reg);
                        free_int.push(b_reg);
                        free_int.push(a_reg);
                        let res = free_int.pop().ok_or("Out of integer registers".to_string())?;
                        self.emit(0x0F);
                        self.emit(0x95); // setnz
                        self.emit_modrm(3, 0, res.encoding());
                        self.emit(0x0F);
                        self.emit(0xB6); // movzx
                        self.emit_modrm(3, res.encoding(), res.encoding());
                        vals.push((res, false));
                    }
                }
                IR::CmpGt => {
                    let (b_reg, b_float) = vals.pop().unwrap();
                    let (a_reg, a_float) = vals.pop().unwrap();
                    if a_float && b_float {
                        self.comisd_reg_reg(b_reg, a_reg);
                        free_float.push(b_reg);
                        free_float.push(a_reg);
                        let res = free_int.pop().ok_or("Out of integer registers".to_string())?;
                        self.emit(0x0F);
                        self.emit(0x96); // setb (b < a => a > b)
                        self.emit_modrm(3, 0, res.encoding());
                        self.emit(0x0F);
                        self.emit(0xB6); // movzx
                        self.emit_modrm(3, res.encoding(), res.encoding());
                        vals.push((res, false));
                    } else {
                        self.cmp_reg_reg(a_reg, b_reg);
                        free_int.push(b_reg);
                        free_int.push(a_reg);
                        let res = free_int.pop().ok_or("Out of integer registers".to_string())?;
                        self.emit(0x0F);
                        self.emit(0x9F); // setg
                        self.emit_modrm(3, 0, res.encoding());
                        self.emit(0x0F);
                        self.emit(0xB6); // movzx
                        self.emit_modrm(3, res.encoding(), res.encoding());
                        vals.push((res, false));
                    }
                }
                IR::CmpLe => {
                    let (b_reg, b_float) = vals.pop().unwrap();
                    let (a_reg, a_float) = vals.pop().unwrap();
                    if a_float && b_float {
                        self.comisd_reg_reg(b_reg, a_reg);
                        free_float.push(b_reg);
                        free_float.push(a_reg);
                        let res = free_int.pop().ok_or("Out of integer registers".to_string())?;
                        self.emit(0x0F);
                        self.emit(0x9E); // setle
                        self.emit_modrm(3, 0, res.encoding());
                        self.emit(0x0F);
                        self.emit(0xB6); // movzx
                        self.emit_modrm(3, res.encoding(), res.encoding());
                        vals.push((res, false));
                    } else {
                        self.cmp_reg_reg(a_reg, b_reg);
                        free_int.push(b_reg);
                        free_int.push(a_reg);
                        let res = free_int.pop().ok_or("Out of integer registers".to_string())?;
                        self.emit(0x0F);
                        self.emit(0x9E); // setle
                        self.emit_modrm(3, 0, res.encoding());
                        self.emit(0x0F);
                        self.emit(0xB6); // movzx
                        self.emit_modrm(3, res.encoding(), res.encoding());
                        vals.push((res, false));
                    }
                }
                IR::CmpGe => {
                    let (b_reg, b_float) = vals.pop().unwrap();
                    let (a_reg, a_float) = vals.pop().unwrap();
                    if a_float && b_float {
                        self.comisd_reg_reg(b_reg, a_reg);
                        free_float.push(b_reg);
                        free_float.push(a_reg);
                        let res = free_int.pop().ok_or("Out of integer registers".to_string())?;
                        self.emit(0x0F);
                        self.emit(0x9D); // setge
                        self.emit_modrm(3, 0, res.encoding());
                        self.emit(0x0F);
                        self.emit(0xB6); // movzx
                        self.emit_modrm(3, res.encoding(), res.encoding());
                        vals.push((res, false));
                    } else {
                        self.cmp_reg_reg(a_reg, b_reg);
                        free_int.push(b_reg);
                        free_int.push(a_reg);
                        let res = free_int.pop().ok_or("Out of integer registers".to_string())?;
                        self.emit(0x0F);
                        self.emit(0x9D); // setge
                        self.emit_modrm(3, 0, res.encoding());
                        self.emit(0x0F);
                        self.emit(0xB6); // movzx
                        self.emit_modrm(3, res.encoding(), res.encoding());
                        vals.push((res, false));
                    }
                }
                IR::And => {
                    let (b_reg, _) = vals.pop().unwrap();
                    let (a_reg, _) = vals.pop().unwrap();
                    self.emit_rex(true, false, false, b_reg.needs_rex());
                    self.emit(0x48 | if b_reg.needs_rex() { 1 } else { 0 });
                    self.emit(0x23); // and r64, r/m64
                    self.emit_modrm(3, b_reg.encoding(), a_reg.encoding());
                    free_int.push(b_reg);
                    vals.push((a_reg, false));
                }
                IR::Or => {
                    let (b_reg, _) = vals.pop().unwrap();
                    let (a_reg, _) = vals.pop().unwrap();
                    self.emit_rex(true, false, false, b_reg.needs_rex());
                    self.emit(0x48 | if b_reg.needs_rex() { 1 } else { 0 });
                    self.emit(0x09); // or r64, r/m64
                    self.emit_modrm(3, b_reg.encoding(), a_reg.encoding());
                    free_int.push(b_reg);
                    vals.push((a_reg, false));
                }
                IR::Not => {
                    let (reg, _) = vals.pop().unwrap();
                    self.emit(0x48);
                    self.emit(0x83);
                    self.emit_modrm(3, 0, reg.encoding()); // cmp r64, 0
                    self.emit_i8(0);
                    self.emit(0x0F);
                    self.emit(0x94); // setz
                    self.emit_modrm(3, 0, reg.encoding());
                    self.emit(0x0F);
                    self.emit(0xB6); // movzx
                    self.emit_modrm(3, 0, reg.encoding());
                    vals.push((reg, false));
                }
                IR::Shl => {
                    let (b_reg, _) = vals.pop().unwrap();
                    let (a_reg, _) = vals.pop().unwrap();
                    self.mov_reg_to_reg(Reg::RCX, b_reg);
                    free_int.push(b_reg);
                    self.emit_rex(true, false, false, a_reg.needs_rex());
                    self.emit(0x48 | if a_reg.needs_rex() { 1 } else { 0 });
                    self.emit(0xD3);
                    self.emit_modrm(3, 4, a_reg.encoding()); // shl r64, cl
                    vals.push((a_reg, false));
                }
                IR::Shr => {
                    let (b_reg, _) = vals.pop().unwrap();
                    let (a_reg, _) = vals.pop().unwrap();
                    self.mov_reg_to_reg(Reg::RCX, b_reg);
                    free_int.push(b_reg);
                    self.emit_rex(true, false, false, a_reg.needs_rex());
                    self.emit(0x48 | if a_reg.needs_rex() { 1 } else { 0 });
                    self.emit(0xD3);
                    self.emit_modrm(3, 5, a_reg.encoding()); // shr r64, cl
                    vals.push((a_reg, false));
                }
                IR::Band => {
                    let (b_reg, _) = vals.pop().unwrap();
                    let (a_reg, _) = vals.pop().unwrap();
                    self.emit_rex(true, false, false, b_reg.needs_rex());
                    self.emit(0x48 | if b_reg.needs_rex() { 1 } else { 0 });
                    self.emit(0x23); // and r64, r/m64
                    self.emit_modrm(3, b_reg.encoding(), a_reg.encoding());
                    free_int.push(b_reg);
                    vals.push((a_reg, false));
                }
                IR::Bor => {
                    let (b_reg, _) = vals.pop().unwrap();
                    let (a_reg, _) = vals.pop().unwrap();
                    self.emit_rex(true, false, false, b_reg.needs_rex());
                    self.emit(0x48 | if b_reg.needs_rex() { 1 } else { 0 });
                    self.emit(0x09); // or r64, r/m64
                    self.emit_modrm(3, b_reg.encoding(), a_reg.encoding());
                    free_int.push(b_reg);
                    vals.push((a_reg, false));
                }
                IR::Bxor => {
                    let (b_reg, _) = vals.pop().unwrap();
                    let (a_reg, _) = vals.pop().unwrap();
                    self.emit_rex(true, false, false, b_reg.needs_rex());
                    self.emit(0x48 | if b_reg.needs_rex() { 1 } else { 0 });
                    self.emit(0x31); // xor r64, r/m64
                    self.emit_modrm(3, b_reg.encoding(), a_reg.encoding());
                    free_int.push(b_reg);
                    vals.push((a_reg, false));
                }
                IR::JmpIfFalse(target) => {
                    let (cond_reg, _) = vals.pop().unwrap();
                    free_int.push(cond_reg);
                    self.emit(0x48);
                    self.emit(0x85);
                    self.emit_modrm(3, cond_reg.encoding(), cond_reg.encoding()); // test reg, reg
                    self.jcc_rel32(0x84, *target); // jz
                }
                IR::Jmp(target) => {
                    self.jmp_rel32(*target);
                }
                IR::Label(idx) => {
                    self.bind_label(*idx);
                }
                IR::Ret => {}
                IR::Drop => {
                    let (reg, is_float) = vals.pop().unwrap();
                    if is_float {
                        free_float.push(reg);
                    } else {
                        free_int.push(reg);
                    }
                }
                IR::Dup => {
                    let top = vals.last().cloned().unwrap();
                    vals.push(top);
                }
                IR::CallNative(_name, _arg_count) => {
                    vals.push((Reg::RAX, false));
                }
                IR::CallFFI(func_addr, arg_count) => {
                    let count = *arg_count;
                    self.sub_imm_from_rsp(SHADOW_SPACE);
                    let mut loaded: Vec<(Reg, bool)> = Vec::new();
                    for _ in 0..count {
                        let (reg, is_float) = vals.pop().unwrap();
                        loaded.push((reg, is_float));
                    }
                    loaded.reverse();
                    let mut int_idx = 0;
                    let mut float_idx = 0;
                    for (reg, is_float) in loaded.iter() {
                        let arg_reg = if *is_float {
                            let r = match float_idx {
                                0 => Reg::XMM0, 1 => Reg::XMM1, 2 => Reg::XMM2, _ => Reg::XMM3,
                            };
                            float_idx += 1;
                            r
                        } else {
                            let r = match int_idx {
                                0 => Reg::RCX, 1 => Reg::RDX, 2 => Reg::R8, _ => Reg::R9,
                            };
                            int_idx += 1;
                            r
                        };
                        if *is_float {
                            self.movsd_reg_reg(arg_reg, *reg);
                            free_float.push(*reg);
                        } else {
                            self.mov_reg_to_reg(arg_reg, *reg);
                            free_int.push(*reg);
                        }
                    }
                    self.mov_imm64_to_reg(Reg::RAX, *func_addr as u64);
                    self.call_abs_reg(Reg::RAX);
                    self.add_imm_to_rsp(SHADOW_SPACE);
                    vals.push((Reg::RAX, false));
                }
                IR::PeekI64 => {
                    let (addr_reg, _) = vals.pop().unwrap();
                    self.mov_mem_to_reg(Reg::RAX, addr_reg, 0);
                    free_int.push(addr_reg);
                    vals.push((Reg::RAX, false));
                }
                IR::PokeI64 => {
                    let (val_reg, _) = vals.pop().unwrap();
                    let (addr_reg, _) = vals.pop().unwrap();
                    free_int.push(val_reg);
                    free_int.push(addr_reg);
                    self.mov_reg_to_mem(addr_reg, 0, val_reg);
                }
                IR::PeekByte => {
                    let (addr_reg, _) = vals.pop().unwrap();
                    self.emit_rex(true, false, false, addr_reg.needs_rex());
                    self.emit(0x48 | if addr_reg.needs_rex() { 1 } else { 0 });
                    self.emit(0x0F);
                    self.emit(0xB6);
                    self.emit_modrm(0, 0, addr_reg.encoding()); // movzx rax, byte [addr]
                    free_int.push(addr_reg);
                    vals.push((Reg::RAX, false));
                }
                IR::PokeByte => {
                    let (val_reg, _) = vals.pop().unwrap();
                    let (addr_reg, _) = vals.pop().unwrap();
                    free_int.push(val_reg);
                    free_int.push(addr_reg);
                    self.emit_rex(true, false, false, addr_reg.needs_rex());
                    self.emit(0x48 | if addr_reg.needs_rex() { 1 } else { 0 });
                    self.emit(0x88);
                    self.emit_modrm(0, 0, addr_reg.encoding()); // mov [addr], al
                }
                IR::MakeClosure(_) => {
                    vals.push((Reg::RAX, false));
                }
                IR::CallClosure => {
                    vals.push((Reg::RAX, false));
                }
                IR::Swap => {
                    let n = vals.len();
                    if n >= 2 {
                        let a = vals.remove(n - 2);
                        let b = vals.remove(n - 1);
                        vals.push(a);
                        vals.push(b);
                    }
                }
                _ => {}
            }
        }

        // --- Epilogue ---
        self.emit(0x48);
        self.emit(0x89);
        self.emit(0xEC); // mov rsp, rbp
        self.emit(0x5D); // pop rbp
        self.emit(0xC3); // ret

        self.resolve_fixups();
        self.buf.finalize()
    }
}
