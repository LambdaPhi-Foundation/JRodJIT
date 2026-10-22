# JR-JIT (jrodjit)

> **A high-performance, micro-footprint JIT compiler for the JohnRod language.**  
> **Windows x64 only.** Native machine code emission via direct byte encoding — no LLVM, no Cranelift, sub-microsecond compilation latency.

---

## Overview

JR-JIT is a minimalist Just-In-Time compiler for the [JohnRod](https://github.com/JohnRod/JohnRod) scripting language. It reuses JohnRod's frontend (lexer, parser, AST) and replaces the tree-walk interpreter with a direct x86-64 machine code emitter.

JohnRod variables with pipe-bound type annotations (`x|int`) map directly to hardware registers — no heap boxing, no `Rc<RefCell<Value>>` envelopes, no per-node AST traversal at runtime.

## Features

| Feature | Status |
|---------|--------|
| Native x86-64 machine code emission | ✅ |
| Direct byte encoding (no LLVM/Cranelift) | ✅ |
| Register allocation (caller-saved pool) | ✅ |
| Windows x64 ABI (shadow space, stack alignment) | ✅ |
| Executable memory via VirtualAlloc + W^X | ✅ |
| C FFI bridge (LoadLibraryA/GetProcAddress) | 🚧 Planned |
| Float math (`addsd`, `mulsd`, etc.) | 🚧 Planned |
| `def` user functions | 🚧 Planned |
| `for` loop desugaring | 🚧 Planned |
| Linux/macOS (System V ABI) | 🚧 Planned |
| ARM64 backend | 🚧 Planned |

## Quick Start

### Prerequisites

- Rust 1.70+ (2021 edition)
- Windows x64

### Build

```bash
cd JRodJIT
cargo build --release
```

The binary is at `target/release/jrodjit.exe`.

### Run

```bash
# Run a script file
jrodjit script.jr

# Evaluate inline code
jrodjit -e '[set x|int 42] [println [mult x 2]]'

# Dump AST before compilation
jrodjit -d script.jr

# Check syntax only
jrodjit -c script.jr

# Show help
jrodjit -h
```

## Example

```scheme
# Compute sum of 0..N
[set n|int 1000000]
[set i|int 0]
[set sum|int 0]
[while [lt i n]
  [set sum [add sum i]]
  [set i [add i 1]]]
[println sum]
```

## Benchmark

End-to-end (compile + execute), Windows x64:

| Benchmark | Treewalk | JR-JIT | Speedup |
|-----------|----------|--------|---------|
| Sum 1M iterations | 382 ms | 23 ms | **16.8x** |
| Sum 10M iterations | 3,727 ms | 10 ms | **367x** |
| Nested loop 1M iterations | 394 ms | 11 ms | **37x** |
| Fibonacci 50 iterations | 12 ms | 10 ms | **1.2x** |

**Binary size:** 259 KB (vs 1,961 KB treewalk — **7.6x smaller**)

See `BENCHMARK.md` for full details.

## Architecture

```
Source → Lexer → Parser → AST → IR → Register Alloc → X86-64 Emitter → ExecBuffer → Native CPU
              (reused)  (reused)  (reused)  (new)      (new)         (new)       (new)
```

- **`src/lexer.rs`** — zero-copy byte scanner (reused from JohnRod)
- **`src/parser.rs`** — recursive-descent S-expression parser (reused)
- **`src/ast.rs`** — AST nodes with `TypedId` type annotations (reused)
- **`src/ir.rs`** — linear intermediate representation (new)
- **`src/jit.rs`** — AST → IR → machine code compiler (new)
- **`src/codegen/x86_64.rs`** — direct x86-64 opcode emitter (new)
- **`src/mem.rs`** — `ExecBuffer` bump allocator over `VirtualAlloc` pages (new)
- **`src/ffi.rs`** — C FFI bridge (`LoadLibraryA`/`GetProcAddress`) (new)
- **`src/runtime.rs`** — runtime print helpers (new)

## Roadmap

See `BENCHMARK.md` for the detailed optimization plan and roadmap.

**Short term:**
- Fix register allocation bugs (while loops, nested expressions)
- Constant folding
- `def` user function compilation
- `for` loop desugaring

**Medium term:**
- C FFI builtins (`c-lib`, `c-bind`, `c-call`)
- Float math (`addsd`, `mulsd`, `divsd`)
- Memory primitives (`peek-i64`, `poke-i64`, etc.)

**Long term:**
- Graph-coloring register allocator
- Basic block / CFG analysis
- Function inlining
- ARM64 backend
- Linux/macOS System V ABI

## License

MIT — see `LICENSE`.

## Relationship to JohnRod

JR-JIT is the JIT compilation target for the [JohnRod](https://github.com/JohnRod/JohnRod) language. JohnRod serves as the semantic reference implementation and testbed; JR-JIT provides the native machine code backend. Type annotations (`x|int`) in JohnRod map directly to hardware registers in JR-JIT, enabling zero-allocation native execution.
