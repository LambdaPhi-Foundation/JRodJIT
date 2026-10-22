use crate::ast::Expr;
use crate::value::Value;
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct FunctionDef {
    pub params: Vec<String>,
    pub param_types: Vec<Option<String>>,
    pub body: Vec<Expr>,
}

pub struct Environment {
    pub vars: FxHashMap<String, Value>,
    pub types: FxHashMap<String, String>,
    pub funcs: FxHashMap<String, FunctionDef>,
    pub closures: FxHashMap<String, Value>,
    pub parent: Option<Rc<RefCell<Environment>>>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            vars: FxHashMap::default(),
            types: FxHashMap::default(),
            funcs: FxHashMap::default(),
            closures: FxHashMap::default(),
            parent: None,
        }
    }

    pub fn with_parent(parent: Rc<RefCell<Environment>>) -> Self {
        Self {
            vars: FxHashMap::default(),
            types: FxHashMap::default(),
            funcs: FxHashMap::default(),
            closures: FxHashMap::default(),
            parent: Some(parent),
        }
    }

    pub fn lookup_var(&self, name: &str) -> Option<Value> {
        if let Some(val) = self.vars.get(name) {
            return Some(val.clone());
        }
        if let Some(ref p) = self.parent {
            return p.borrow().lookup_var(name);
        }
        None
    }

    pub fn set_var(&mut self, name: String, val: Value) {
        self.vars.insert(name, val);
    }

    #[inline(always)]
    pub fn lookup_type(&self, name: &str) -> Option<String> {
        if let Some(ty) = self.types.get(name) {
            return Some(ty.clone());
        }
        if let Some(ref p) = self.parent {
            return p.borrow().lookup_type(name);
        }
        None
    }

    #[inline(always)]
    pub fn set_type(&mut self, name: String, ty: String) {
        self.types.insert(name, ty);
    }

    pub fn lookup_func(&self, name: &str) -> Option<FunctionDef> {
        if let Some(f) = self.funcs.get(name) {
            return Some(f.clone());
        }
        if let Some(ref p) = self.parent {
            return p.borrow().lookup_func(name);
        }
        None
    }

    pub fn set_func(&mut self, name: String, f: FunctionDef) {
        self.funcs.insert(name, f);
    }

    pub fn lookup_closure(&self, name: &str) -> Option<Value> {
        if let Some(v) = self.closures.get(name) {
            return Some(v.clone());
        }
        if let Some(ref p) = self.parent {
            return p.borrow().lookup_closure(name);
        }
        None
    }

    pub fn set_closure(&mut self, name: String, v: Value) {
        self.closures.insert(name, v);
    }

    pub fn clear(&mut self) {
        self.vars.clear();
        self.types.clear();
        self.funcs.clear();
        self.closures.clear();
        self.parent = None;
    }
}
