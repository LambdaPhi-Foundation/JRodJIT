#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Id(String),
    TypedId(String, String),
    List(Vec<Expr>),
    Array(Vec<Expr>),
    Object(Vec<(Expr, Expr)>),
    Comment(String),
    Quote(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub line: usize,
    pub col: usize,
}

impl Expr {
    #[inline(always)]
    pub fn new(kind: ExprKind, line: usize, col: usize) -> Self {
        Self { kind, line, col }
    }
}

pub fn expr_to_string(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::Int(v) => v.to_string(),
        ExprKind::Float(v) => {
            if v.fract() == 0.0 && !v.is_infinite() && !v.is_nan() {
                format!("{:.1}", v)
            } else {
                v.to_string()
            }
        }
        ExprKind::String(v) => format!("{:?}", v),
        ExprKind::Bool(v) => v.to_string(),
        ExprKind::Id(v) => v.clone(),
        ExprKind::TypedId(name, ty) => format!("{}|{}", name, ty),
        ExprKind::List(children) => {
            let inner = children.iter().map(expr_to_string).collect::<Vec<_>>().join(" ");
            format!("[{}]", inner)
        }
        ExprKind::Array(children) => {
            let inner = children.iter().map(expr_to_string).collect::<Vec<_>>().join(" ");
            format!("[{}]", inner)
        }
        ExprKind::Object(pairs) => {
            let inner = pairs
                .iter()
                .map(|(k, v)| format!("{}: {}", expr_to_string(k), expr_to_string(v)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{{}}}", inner)
        }
        ExprKind::Comment(v) => format!("#{}", v),
        ExprKind::Quote(inner) => format!("'{}", expr_to_string(inner)),
    }
}

pub fn dump_ast(expr: &Expr, indent: usize) {
    let space = "  ".repeat(indent);
    match &expr.kind {
        ExprKind::Int(v) => println!("{}Int: {}", space, v),
        ExprKind::Float(v) => println!("{}Float: {}", space, v),
        ExprKind::String(v) => println!("{}Str: {}", space, v),
        ExprKind::Bool(v) => println!("{}Bool: {}", space, v),
        ExprKind::Id(v) => println!("{}Id: {}", space, v),
        ExprKind::TypedId(name, ty) => println!("{}TypedId: {}|{}", space, name, ty),
        ExprKind::List(children) => {
            println!("{}List:", space);
            for child in children {
                dump_ast(child, indent + 1);
            }
        }
        ExprKind::Array(children) => {
            println!("{}Array:", space);
            for child in children {
                dump_ast(child, indent + 1);
            }
        }
        ExprKind::Object(pairs) => {
            println!("{}Object:", space);
            for (k, v) in pairs {
                dump_ast(k, indent + 1);
                dump_ast(v, indent + 2);
            }
        }
        ExprKind::Comment(v) => println!("{}Comment: {}", space, v),
        ExprKind::Quote(inner) => {
            println!("{}Quote:", space);
            dump_ast(inner, indent + 1);
        }
    }
}
