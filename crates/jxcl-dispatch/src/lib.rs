//! Table-driven opcode-to-handler dispatch, extracted from the fetch/decode/execute loop's
//! match statement so dispatch can be tested and extended independently of execution semantics.
//!
//! Owns: The dispatch table (Opcode -> handler fn pointer). Provides a fast, testable
//! mapping from opcode mnemonics to their handler routines.
#![forbid(unsafe_code)]

use jxcl_opcodes::Mnemonic;

/// A dispatch handler for a single opcode. The return value is a boolean indicating
/// whether execution should continue (false) or halt (true).
pub type DispatchHandler = fn() -> bool;

/// The dispatch table maps opcodes to their handler functions.
/// This enables decoupling the dispatch logic from the actual instruction execution.
pub struct DispatchTable {
    handlers: &'static [(Mnemonic, DispatchHandler)],
}

impl DispatchTable {
    /// Create a new dispatch table with the given handler mapping.
    pub const fn new(handlers: &'static [(Mnemonic, DispatchHandler)]) -> Self {
        DispatchTable { handlers }
    }

    /// Look up the handler for a given mnemonic.
    /// Returns Some(&handler) if found, None if the opcode is unknown.
    pub fn lookup(&self, mnemonic: Mnemonic) -> Option<DispatchHandler> {
        for (mnem, handler) in self.handlers {
            if *mnem == mnemonic {
                return Some(*handler);
            }
        }
        None
    }

    /// Check if a mnemonic is registered in the dispatch table.
    pub fn contains(&self, mnemonic: Mnemonic) -> bool {
        self.lookup(mnemonic).is_some()
    }

    /// Get the number of registered handlers.
    pub const fn len(&self) -> usize {
        self.handlers.len()
    }

    /// Check if the dispatch table is empty.
    pub const fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

/// A placeholder handler that does nothing (useful for testing or NOPs).
pub fn nop_handler() -> bool {
    true
}

/// A handler that halts execution.
pub fn halt_handler() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_table_lookup() {
        let handlers = &[(Mnemonic::Nop, nop_handler as DispatchHandler),
                         (Mnemonic::Halt, halt_handler as DispatchHandler)];
        let table = DispatchTable::new(handlers);

        assert!(table.lookup(Mnemonic::Nop).is_some());
        assert!(table.lookup(Mnemonic::Halt).is_some());
        assert!(table.lookup(Mnemonic::Add).is_none());
    }

    #[test]
    fn dispatch_table_contains() {
        let handlers = &[(Mnemonic::Nop, nop_handler as DispatchHandler)];
        let table = DispatchTable::new(handlers);

        assert!(table.contains(Mnemonic::Nop));
        assert!(!table.contains(Mnemonic::Add));
    }

    #[test]
    fn dispatch_table_len() {
        let handlers = &[(Mnemonic::Nop, nop_handler as DispatchHandler),
                         (Mnemonic::Halt, halt_handler as DispatchHandler)];
        let table = DispatchTable::new(handlers);

        assert_eq!(table.len(), 2);
    }

    #[test]
    fn empty_dispatch_table() {
        let table = DispatchTable::new(&[]);
        assert!(table.is_empty());
        assert_eq!(table.len(), 0);
        assert!(table.lookup(Mnemonic::Nop).is_none());
    }

    #[test]
    fn nop_handler_returns_true() {
        assert!(nop_handler());
    }

    #[test]
    fn halt_handler_returns_false() {
        assert!(!halt_handler());
    }
}
