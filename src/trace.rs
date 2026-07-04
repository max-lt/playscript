use std::fmt;

use crate::value::Value;

/// One recorded step of execution. `op` is the operations-clock value at the
/// moment it happened — the reproducible timeline position — and `depth` is
/// the call depth, for indentation and call-tree reconstruction.
#[derive(Debug, Clone)]
pub struct TraceEvent {
    pub op: u64,
    pub depth: usize,
    /// 1-based source line the event came from — for editor highlighting.
    pub line: usize,
    pub kind: EventKind,
}

/// What a trace event records. The trace captures state changes and control
/// flow — assignments, calls, returns, branch decisions — not every node, so
/// it stays legible: it is meant to be *read*, by a person or a visualizer.
#[derive(Debug, Clone)]
pub enum EventKind {
    /// A variable or array element was bound or updated.
    Assign { target: String, value: Value },
    /// A user function was entered.
    Call { name: String, args: Vec<Value> },
    /// A user function returned.
    Return { name: String, value: Value },
    /// An `if` or `while` condition was evaluated (`construct` is which one).
    Branch { construct: &'static str, value: bool },
}

impl fmt::Display for EventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {

        match self {
            EventKind::Assign { target, value } => write!(f, "{target} = {value}"),
            EventKind::Call { name, args } => {
                write!(f, "call {name}(")?;

                for (i, arg) in args.iter().enumerate() {

                    if i > 0 {
                        write!(f, ", ")?;
                    }

                    write!(f, "{arg}")?;
                }

                write!(f, ")")
            }
            EventKind::Return { name, value } => write!(f, "return {name} → {value}"),
            EventKind::Branch { construct, value } => write!(f, "{construct} → {value}"),
        }
    }
}
