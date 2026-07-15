//! Resolved types used during semantic analysis.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    Void,
    Bool,
    Char,
    Int,
    Long,
    LongLong,
    UInt,
    ULong,
    ULongLong,
    Float,
    Double,
    Auto,
    /// Named class / typedef / template specialization.
    Named {
        name: String,
        args: Vec<Ty>,
    },
    Pointer(Box<Ty>),
    Reference(Box<Ty>),
    Const(Box<Ty>),
    /// Function type for call checking.
    Function {
        ret: Box<Ty>,
        params: Vec<Ty>,
    },
    /// Fallback when we cannot resolve further (still allows some progress).
    Unknown,
    Error,
}

impl Ty {
    pub fn strip_cv_ref(&self) -> &Ty {
        match self {
            Ty::Const(t) | Ty::Reference(t) => t.strip_cv_ref(),
            other => other,
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(
            self.strip_cv_ref(),
            Ty::Bool
                | Ty::Char
                | Ty::Int
                | Ty::Long
                | Ty::LongLong
                | Ty::UInt
                | Ty::ULong
                | Ty::ULongLong
                | Ty::Float
                | Ty::Double
        )
    }

    pub fn is_integral(&self) -> bool {
        matches!(
            self.strip_cv_ref(),
            Ty::Bool
                | Ty::Char
                | Ty::Int
                | Ty::Long
                | Ty::LongLong
                | Ty::UInt
                | Ty::ULong
                | Ty::ULongLong
        )
    }

    pub fn named(name: impl Into<String>, args: Vec<Ty>) -> Self {
        Ty::Named {
            name: name.into(),
            args,
        }
    }
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::Void => write!(f, "void"),
            Ty::Bool => write!(f, "bool"),
            Ty::Char => write!(f, "char"),
            Ty::Int => write!(f, "int"),
            Ty::Long => write!(f, "long"),
            Ty::LongLong => write!(f, "long long"),
            Ty::UInt => write!(f, "unsigned int"),
            Ty::ULong => write!(f, "unsigned long"),
            Ty::ULongLong => write!(f, "unsigned long long"),
            Ty::Float => write!(f, "float"),
            Ty::Double => write!(f, "double"),
            Ty::Auto => write!(f, "auto"),
            Ty::Named { name, args } => {
                write!(f, "{name}")?;
                if !args.is_empty() {
                    write!(f, "<")?;
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{a}")?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }
            Ty::Pointer(t) => write!(f, "{t}*"),
            Ty::Reference(t) => write!(f, "{t}&"),
            Ty::Const(t) => write!(f, "const {t}"),
            Ty::Function { ret, params } => {
                write!(f, "{ret}(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{p}")?;
                }
                write!(f, ")")
            }
            Ty::Unknown => write!(f, "<?>"),
            Ty::Error => write!(f, "<error>"),
        }
    }
}
