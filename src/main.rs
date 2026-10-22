use jrodjit::ast::dump_ast;
use jrodjit::jit::JITCompiler;
use jrodjit::lexer::tokenize;
use jrodjit::parser::parse;
use jrodjit::runtime::{jr_print_bool, jr_print_float, jr_print_int, jr_print_ptr};
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::process;

fn print_help() {
    println!(
        r#"JRodJIT - Just-In-Time Compiler for JohnRod - Version 0.1.0

USAGE:
  jrodjit [options] [file.jr]

OPTIONS:
  -h, --help       Show this help message
  -v, --version    Show version information
  -e, --eval       Evaluate expression directly from command line
  -d, --dump       Dump AST before compilation (debug)
  -c, --check      Check syntax only (no compilation)

EXAMPLES:
  jrodjit script.jr                    # JIT-compile and run a script file
  jrodjit -e '[set x|int 42] [println [mult x 2]]'  # Evaluate inline
  jrodjit -d script.jr                 # Dump AST and run
  jrodjit -c script.jr                 # Check syntax only

FILE EXTENSIONS:
  .jr     - JohnRod source file

NOTE: Windows x64 only. Core types: int, float, bool, ptr, void.
Heavy features (HTTP, JSON, file I/O) available via C FFI."#
    );
    process::exit(0);
}

fn print_version() {
    println!(
        r#"JRodJIT - Version 0.1.0
Windows x64 JIT Compiler
Built with Rust (Aggressive Optimization)"#
    );
    process::exit(0);
}

fn display_error(source: &str, err_msg: &str, filename: &str) {
    let mut line_num: Option<usize> = None;
    let mut clean_msg = err_msg.to_string();

    let lower = err_msg.to_ascii_lowercase();
    if let Some(pos) = lower.find("line") {
        let remainder = &err_msg[pos + 4..];
        let trimmed = remainder.trim_start_matches(|c: char| c.is_whitespace() || c == ':');
        let num_str: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(num) = num_str.parse::<usize>() {
            line_num = Some(num);
            if let Some(colon_pos) = trimmed.find(':') {
                clean_msg = trimmed[colon_pos + 1..].trim().to_string();
            }
        }
    }

    if let Some(l) = line_num {
        let lines: Vec<&str> = source.lines().collect();
        if l > 0 && l <= lines.len() {
            println!("Error at line {}: {}", l, clean_msg);
            let line_content = lines[l - 1].trim();
            println!("{}", line_content);
            println!("{}^", " ".repeat(line_content.len()));
            return;
        }
    }

    if filename != "<stdin>" {
        println!("Error in {}: {}", filename, err_msg);
    } else {
        println!("Error: {}", err_msg);
    }
}

fn register_runtime_functions(compiler: &mut JITCompiler) {
    compiler.register_runtime(
        "jr_print_int",
        unsafe { std::mem::transmute::<extern "C" fn(i64), usize>(jr_print_int) },
    );
    compiler.register_runtime(
        "jr_print_float",
        unsafe { std::mem::transmute::<extern "C" fn(f64), usize>(jr_print_float) },
    );
    compiler.register_runtime(
        "jr_print_ptr",
        unsafe { std::mem::transmute::<extern "C" fn(*mut std::ffi::c_void), usize>(jr_print_ptr) },
    );
    compiler.register_runtime(
        "jr_print_bool",
        unsafe { std::mem::transmute::<extern "C" fn(i8), usize>(jr_print_bool) },
    );
}

fn run_script(filename: &str, source: &str, dump_ast_flag: bool, check_only: bool) {
    let tokens = match tokenize(source) {
        Ok(toks) => toks,
        Err(e) => {
            display_error(source, &e, filename);
            process::exit(1);
        }
    };

    let ast = match parse(tokens) {
        Ok(a) => a,
        Err(e) => {
            display_error(source, &e, filename);
            process::exit(1);
        }
    };

    if dump_ast_flag {
        println!("=== Abstract Syntax Tree ===");
        for node in &ast {
            dump_ast(node, 0);
        }
        println!("============================");
        if check_only {
            println!("Syntax check passed");
            return;
        }
    }

    if check_only {
        println!("Syntax check passed");
        return;
    }

    let mut compiler = match JITCompiler::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: JIT compiler init failed: {}", e);
            process::exit(1);
        }
    };

    register_runtime_functions(&mut compiler);

    let entry = match compiler.compile_program(&ast) {
        Ok(entry) => entry,
        Err(e) => {
            eprintln!("Error: JIT compilation failed: {}", e);
            process::exit(1);
        }
    };

    if entry.is_null() {
        eprintln!("Error: JIT produced null entry point");
        process::exit(1);
    }
    // Call via raw function pointer cast
    let main_fn: extern "C" fn() -> i64 = unsafe { std::mem::transmute(entry as usize) };
    let _result = unsafe { main_fn() };
}

fn run_repl() {
    println!("JRodJIT REPL - Type expressions to JIT-compile (Ctrl+C to exit)");
    println!("Type 'help' for commands\n");

    let stdin = io::stdin();
    let mut reader = stdin.lock();

    loop {
        print!("> ");
        if io::stdout().flush().is_err() {
            break;
        }

        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                println!("Goodbye!");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                match trimmed {
                    "help" => {
                        println!(
                            r#"Available REPL commands:
  help    - Show this help
  exit    - Exit REPL
  ast     - Show AST before compilation"#
                        );
                        continue;
                    }
                    "exit" | "quit" => {
                        println!("Goodbye!");
                        process::exit(0);
                    }
                    "" => continue,
                    _ => {}
                }

                match tokenize(trimmed) {
                    Ok(toks) => match parse(toks) {
                        Ok(ast) => {
                            println!("AST:");
                            for node in &ast {
                                dump_ast(node, 0);
                            }
                            let mut compiler = match JITCompiler::new() {
                                Ok(c) => c,
                                Err(e) => {
                                    println!("JIT Init Error: {}", e);
                                    continue;
                                }
                            };
                            register_runtime_functions(&mut compiler);
                            match compiler.compile_program(&ast) {
                                Ok(entry) => {
                                    if !entry.is_null() {
                                        type MainFn = extern "C" fn() -> i64;
                                        let main_fn: MainFn =
                                            unsafe { std::mem::transmute(entry) };
                                        let _ = unsafe { main_fn() };
                                    }
                                }
                                Err(e) => println!("JIT Error: {}", e),
                            }
                        }
                        Err(e) => println!("Error: {}", e),
                    },
                    Err(e) => println!("Error: {}", e),
                }
            }
            Err(_) => {
                println!("Goodbye!");
                break;
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        run_repl();
        return;
    }

    let mut i = 0;
    let mut filename = String::new();
    let mut eval_expr = String::new();
    let mut dump = false;
    let mut check = false;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "-h" | "--help" => print_help(),
            "-v" | "--version" => print_version(),
            "-d" | "--dump" => {
                dump = true;
                i += 1;
            }
            "-c" | "--check" => {
                check = true;
                i += 1;
            }
            "-e" | "--eval" => {
                if i + 1 < args.len() {
                    eval_expr = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: -e requires an expression");
                    process::exit(1);
                }
            }
            _ => {
                filename = arg.clone();
                i += 1;
            }
        }
    }

    if !eval_expr.is_empty() {
        match tokenize(&eval_expr) {
            Ok(tokens) => match parse(tokens) {
                Ok(ast) => {
                    if dump {
                        println!("=== Abstract Syntax Tree ===");
                        for node in &ast {
                            dump_ast(node, 0);
                        }
                        println!("============================");
                    }
                    if check {
                        println!("Syntax check passed");
                        return;
                    }

                    let mut compiler = match JITCompiler::new() {
                        Ok(c) => c,
                        Err(e) => {
                            eprintln!("JIT Init Error: {}", e);
                            process::exit(1);
                        }
                    };
                    register_runtime_functions(&mut compiler);
                    match compiler.compile_program(&ast) {
                        Ok(entry) => {
                            if !entry.is_null() {
                                type MainFn = extern "C" fn() -> i64;
                                let main_fn: MainFn =
                                    unsafe { std::mem::transmute(entry) };
                                let _ = unsafe { main_fn() };
                            }
                        }
                        Err(e) => {
                            eprintln!("JIT Error: {}", e);
                            process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
        return;
    }

    if !filename.is_empty() {
        match fs::read_to_string(&filename) {
            Ok(source) => {
                run_script(&filename, &source, dump, check);
            }
            Err(e) => {
                eprintln!("Error: Failed to read file '{}': {}", filename, e);
                process::exit(1);
            }
        }
    } else {
        print_help();
    }
}
