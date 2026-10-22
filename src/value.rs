
use indexmap::IndexMap;
use rustc_hash::FxBuildHasher;
use std::cell::RefCell;
use std::rc::Rc;

pub type ObjectMap = IndexMap<String, Value, FxBuildHasher>;

#[derive(Clone, Debug)]
pub enum Value {
    Void,
    Int(i64),
    Float(f64),
    Bool(bool),
    Ptr(usize),
    String(Rc<String>),
    Array(Rc<RefCell<Vec<Value>>>),
    Object(Rc<RefCell<ObjectMap>>),
    Closure(Rc<RefCell<CompiledFunction>>),
}

#[derive(Clone, Debug)]
pub struct CompiledFunction {
    pub entry: *mut u8,
    pub param_count: usize,
    pub param_types: Vec<String>,
    pub ret_type: String,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Void, Value::Void) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) => (*a as f64) == *b,
            (Value::Float(a), Value::Int(b)) => *a == (*b as f64),
            (Value::String(a), Value::String(b)) => Rc::ptr_eq(a, b) || a == b,
            (Value::Ptr(a), Value::Ptr(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => Rc::ptr_eq(a, b) || *a.borrow() == *b.borrow(),
            (Value::Object(a), Value::Object(b)) => Rc::ptr_eq(a, b) || *a.borrow() == *b.borrow(),
            (Value::Closure(a), Value::Closure(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl Value {
    #[inline(always)]
    pub fn new_int(v: i64) -> Self {
        Value::Int(v)
    }

    #[inline(always)]
    pub fn new_float(v: f64) -> Self {
        Value::Float(v)
    }

    #[inline(always)]
    pub fn new_string<S: Into<String>>(v: S) -> Self {
        Value::String(Rc::new(v.into()))
    }

    #[inline(always)]
    pub fn new_bool(v: bool) -> Self {
        Value::Bool(v)
    }

    #[inline(always)]
    pub fn new_void() -> Self {
        Value::Void
    }

    #[inline(always)]
    pub fn new_ptr(p: usize) -> Self {
        Value::Ptr(p)
    }

    #[inline(always)]
    pub fn new_array() -> Self {
        Value::Array(Rc::new(RefCell::new(Vec::new())))
    }

    #[inline(always)]
    pub fn new_array_with(elements: Vec<Value>) -> Self {
        Value::Array(Rc::new(RefCell::new(elements)))
    }

    #[inline(always)]
    pub fn new_object() -> Self {
        Value::Object(Rc::new(RefCell::new(IndexMap::with_hasher(
            FxBuildHasher::default(),
        ))))
    }

    #[inline(always)]
    pub fn new_closure(entry: *mut u8, param_count: usize, param_types: Vec<String>, ret_type: String) -> Self {
        Value::Closure(Rc::new(RefCell::new(CompiledFunction {
            entry,
            param_count,
            param_types,
            ret_type,
        })))
    }

    #[inline(always)]
    pub fn is_int(&self) -> bool {
        matches!(self, Value::Int(_))
    }

    #[inline(always)]
    pub fn is_float(&self) -> bool {
        matches!(self, Value::Float(_))
    }

    #[inline(always)]
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Int(_) | Value::Float(_))
    }

    #[inline(always)]
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    #[inline(always)]
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    #[inline(always)]
    pub fn is_void(&self) -> bool {
        matches!(self, Value::Void)
    }

    #[inline(always)]
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    #[inline(always)]
    pub fn is_object(&self) -> bool {
        matches!(self, Value::Object(_))
    }

    #[inline(always)]
    pub fn is_ptr(&self) -> bool {
        matches!(self, Value::Ptr(_))
    }

    #[inline(always)]
    pub fn is_closure(&self) -> bool {
        matches!(self, Value::Closure(_))
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Void => "void",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Ptr(_) => "ptr",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
            Value::Closure(_) => "closure",
        }
    }

    pub fn as_int(&self) -> Result<i64, String> {
        match self {
            Value::Int(v) => Ok(*v),
            _ => Err(format!("Expected int, got {}", self.type_name())),
        }
    }

    pub fn as_float(&self) -> Result<f64, String> {
        match self {
            Value::Int(v) => Ok(*v as f64),
            Value::Float(v) => Ok(*v),
            _ => Err(format!("Expected number, got {}", self.type_name())),
        }
    }

    pub fn as_string(&self) -> Result<String, String> {
        match self {
            Value::String(s) => Ok(s.to_string()),
            _ => Err(format!("Expected string, got {}", self.type_name())),
        }
    }

    pub fn as_bool(&self) -> Result<bool, String> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err(format!("Expected bool, got {}", self.type_name())),
        }
    }

    pub fn as_ptr(&self) -> Result<usize, String> {
        match self {
            Value::Ptr(p) => Ok(*p),
            _ => Err(format!("Expected ptr, got {}", self.type_name())),
        }
    }

    pub fn as_array(&self) -> Result<Rc<RefCell<Vec<Value>>>, String> {
        match self {
            Value::Array(a) => Ok(Rc::clone(a)),
            _ => Err(format!("Expected array, got {}", self.type_name())),
        }
    }

    pub fn as_object(&self) -> Result<Rc<RefCell<ObjectMap>>, String> {
        match self {
            Value::Object(o) => Ok(Rc::clone(o)),
            _ => Err(format!("Expected object, got {}", self.type_name())),
        }
    }

    pub fn as_closure(&self) -> Result<Rc<RefCell<CompiledFunction>>, String> {
        match self {
            Value::Closure(c) => Ok(Rc::clone(c)),
            _ => Err(format!("Expected closure, got {}", self.type_name())),
        }
    }

    pub fn to_string_repr(&self) -> String {
        match self {
            Value::Int(v) => v.to_string(),
            Value::Float(v) => {
                if v.fract() == 0.0 && !v.is_infinite() && !v.is_nan() {
                    format!("{:.1}", v)
                } else {
                    v.to_string()
                }
            }
            Value::String(s) => s.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Void => "void".to_string(),
            Value::Ptr(p) => format!("0x{:x}", p),
            Value::Array(a) => format!("array[{}]", a.borrow().len()),
            Value::Object(_) => "object".to_string(),
            Value::Closure(_) => "closure".to_string(),
        }
    }

    pub fn check_type(&self, expected: &str) -> bool {
        match expected {
            "int" => self.is_int(),
            "float" => self.is_float(),
            "number" => self.is_number(),
            "str" | "string" => self.is_string(),
            "bool" => self.is_bool(),
            "array" => self.is_array(),
            "object" => self.is_object(),
            "ptr" => self.is_ptr(),
            "closure" => self.is_closure(),
            "void" => self.is_void(),
            _ => true,
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Void
    }
}
