//! AST -> MIR lowering: turns structured control flow (`if`/`while`)
//! into an explicit control-flow graph, and nested expressions into a
//! flat sequence of single-operation assignments to fresh temporaries.
//!
//! ## Fails closed, never panics
//!
//! `lower_program` returns a [`LowerError`] for anything it can't
//! make sense of (an unbound variable, a call to an undeclared
//! function, `result` used outside `ensures`) rather than assuming
//! the input is well-formed. This is *not* a type checker -- it
//! doesn't check that operators are applied to operands of matching
//! `Ty`s -- but it never silently drops or misinterprets a construct
//! it doesn't recognize.
use crate::ast::{self, BinOp, Program as AstProgram, Ty, UnOp};
use crate::mir::{
    BasicBlock, BlockId, Const, Function, Local, LocalDecl, Operand, Program, Rvalue, Statement,
    Terminator,
};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LowerError {
    UnboundVariable(String),
    UnknownFunction(String),
    /// `result` appeared somewhere other than an `ensures`/`requires`
    /// expression (those are lowered separately, kept as surface AST
    /// -- see `mir::Function`'s doc comment).
    ResultOutsideContract,
    DuplicateFunction(String),
}

impl std::fmt::Display for LowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LowerError::UnboundVariable(name) => write!(f, "unbound variable `{name}`"),
            LowerError::UnknownFunction(name) => write!(f, "call to undeclared function `{name}`"),
            LowerError::ResultOutsideContract => {
                write!(
                    f,
                    "`result` may only be used inside a `requires`/`ensures` clause"
                )
            }
            LowerError::DuplicateFunction(name) => {
                write!(f, "function `{name}` is declared more than once")
            }
        }
    }
}
impl std::error::Error for LowerError {}

struct Builder<'a> {
    known_functions: &'a std::collections::HashSet<String>,
    locals: Vec<LocalDecl>,
    blocks: Vec<BasicBlock>,
    current: BlockId,
    vars: HashMap<String, Local>,
    loop_invariants: BTreeMap<u32, Vec<ast::Expr>>,
    loop_decreases: BTreeMap<u32, ast::Expr>,
}

impl<'a> Builder<'a> {
    fn new_local(&mut self, ty: Ty, name: Option<String>) -> Local {
        let id = Local(self.locals.len() as u32);
        self.locals.push(LocalDecl { ty, name });
        id
    }

    fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.blocks.len() as u32);
        self.blocks.push(BasicBlock {
            statements: Vec::new(),
            terminator: Terminator::Unreachable,
        });
        id
    }

    fn push_stmt(&mut self, stmt: Statement) {
        self.blocks[self.current.0 as usize].statements.push(stmt);
    }

    fn set_terminator(&mut self, block: BlockId, term: Terminator) {
        self.blocks[block.0 as usize].terminator = term;
    }

    /// Lower `e`, pushing whatever temporary assignments it needs into
    /// the current block, and return an [`Operand`] referring to its
    /// value.
    fn lower_expr(&mut self, e: &ast::Expr) -> Result<Operand, LowerError> {
        match e {
            ast::Expr::IntLit(n, _) => Ok(Operand::Const(Const::Int(*n))),
            ast::Expr::BoolLit(b, _) => Ok(Operand::Const(Const::Bool(*b))),
            ast::Expr::Result(_) => Err(LowerError::ResultOutsideContract),
            ast::Expr::Var(name, _) => {
                let local = self
                    .vars
                    .get(name)
                    .copied()
                    .ok_or_else(|| LowerError::UnboundVariable(name.clone()))?;
                Ok(Operand::Copy(local))
            }
            ast::Expr::Unary(op, inner, _) => {
                let inner_op = self.lower_expr(inner)?;
                let ty = self.ty_of_unary(*op);
                let tmp = self.new_local(ty, None);
                self.push_stmt(Statement::Assign(tmp, Rvalue::UnaryOp(*op, inner_op)));
                Ok(Operand::Copy(tmp))
            }
            ast::Expr::Binary(op, lhs, rhs, _) => {
                let lhs_op = self.lower_expr(lhs)?;
                let rhs_op = self.lower_expr(rhs)?;
                let ty = self.ty_of_binary(*op);
                let tmp = self.new_local(ty, None);
                self.push_stmt(Statement::Assign(
                    tmp,
                    Rvalue::BinaryOp(*op, lhs_op, rhs_op),
                ));
                Ok(Operand::Copy(tmp))
            }
            ast::Expr::Call(name, args, _) => {
                if !self.known_functions.contains(name) {
                    return Err(LowerError::UnknownFunction(name.clone()));
                }
                // Calls are evaluated for their argument side effects
                // only here (no interprocedural lowering yet -- see
                // this crate's top-level doc comment on scope); the
                // result is modeled as an opaque fresh value.
                for a in args {
                    self.lower_expr(a)?;
                }
                let tmp = self.new_local(Ty::I64, None);
                Ok(Operand::Copy(tmp))
            }
        }
    }

    fn ty_of_unary(&self, op: UnOp) -> Ty {
        match op {
            UnOp::Neg => Ty::I64,
            UnOp::Not => Ty::Bool,
        }
    }

    fn ty_of_binary(&self, op: BinOp) -> Ty {
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => Ty::I64,
            BinOp::Eq
            | BinOp::Ne
            | BinOp::Lt
            | BinOp::Le
            | BinOp::Gt
            | BinOp::Ge
            | BinOp::And
            | BinOp::Or => Ty::Bool,
        }
    }

    fn lower_block(&mut self, block: &ast::Block) -> Result<(), LowerError> {
        for stmt in &block.stmts {
            self.lower_stmt(stmt)?;
        }
        Ok(())
    }

    fn lower_stmt(&mut self, stmt: &ast::Stmt) -> Result<(), LowerError> {
        match stmt {
            ast::Stmt::Let {
                name, ty, value, ..
            } => {
                let op = self.lower_expr(value)?;
                let declared_ty = ty.unwrap_or(self.ty_of_operand_hint(&op));
                let local = self.new_local(declared_ty, Some(name.clone()));
                self.push_stmt(Statement::Assign(local, Rvalue::Use(op)));
                self.vars.insert(name.clone(), local);
                Ok(())
            }
            ast::Stmt::Assign { name, value, .. } => {
                let op = self.lower_expr(value)?;
                let local = self
                    .vars
                    .get(name)
                    .copied()
                    .ok_or_else(|| LowerError::UnboundVariable(name.clone()))?;
                self.push_stmt(Statement::Assign(local, Rvalue::Use(op)));
                Ok(())
            }
            ast::Stmt::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                let cond_op = self.lower_expr(cond)?;
                let then_block = self.new_block();
                let else_block = self.new_block();
                let join_block = self.new_block();
                self.set_terminator(
                    self.current,
                    Terminator::SwitchInt {
                        discr: cond_op,
                        then_block,
                        else_block,
                    },
                );

                self.current = then_block;
                self.lower_block(then_branch)?;
                self.set_terminator(self.current, Terminator::Goto(join_block));

                self.current = else_block;
                if let Some(else_branch) = else_branch {
                    self.lower_block(else_branch)?;
                }
                self.set_terminator(self.current, Terminator::Goto(join_block));

                self.current = join_block;
                Ok(())
            }
            ast::Stmt::While {
                cond,
                invariants,
                decreases,
                body,
                ..
            } => {
                let header = self.new_block();
                self.set_terminator(self.current, Terminator::Goto(header));
                self.current = header;

                if !invariants.is_empty() {
                    self.loop_invariants.insert(header.0, invariants.clone());
                }
                if let Some(measure) = decreases {
                    self.loop_decreases.insert(header.0, measure.clone());
                }

                let cond_op = self.lower_expr(cond)?;
                let body_block = self.new_block();
                let exit_block = self.new_block();
                // Re-fetch `header`'s terminator target: the condition
                // may have pushed statements into `header` itself
                // (still the current block), so the switch belongs on
                // `self.current`, which at this point is still `header`.
                self.set_terminator(
                    self.current,
                    Terminator::SwitchInt {
                        discr: cond_op,
                        then_block: body_block,
                        else_block: exit_block,
                    },
                );

                self.current = body_block;
                self.lower_block(body)?;
                self.set_terminator(self.current, Terminator::Goto(header));

                self.current = exit_block;
                Ok(())
            }
            ast::Stmt::Return(value, _) => {
                let op = match value {
                    Some(e) => self.lower_expr(e)?,
                    None => Operand::Const(Const::Int(0)),
                };
                self.push_stmt(Statement::Assign(RET_LOCAL, Rvalue::Use(op)));
                self.set_terminator(self.current, Terminator::Return);
                // Anything lexically following an unconditional return
                // is dead code; give it its own (unreachable) block
                // rather than mixing it into an already-terminated one.
                self.current = self.new_block();
                Ok(())
            }
            ast::Stmt::Expr(e) => {
                self.lower_expr(e)?;
                Ok(())
            }
        }
    }

    fn ty_of_operand_hint(&self, op: &Operand) -> Ty {
        match op {
            Operand::Const(Const::Int(_)) => Ty::I64,
            Operand::Const(Const::Bool(_)) => Ty::Bool,
            Operand::Copy(local) => self.locals[local.0 as usize].ty,
        }
    }
}

/// The return place is always local 0, exactly like rustc MIR's `_0`.
const RET_LOCAL: Local = Local(0);

fn lower_function(
    f: &ast::Function,
    known_functions: &std::collections::HashSet<String>,
) -> Result<Function, LowerError> {
    let mut builder = Builder {
        known_functions,
        locals: Vec::new(),
        blocks: Vec::new(),
        current: BlockId(0),
        vars: HashMap::new(),
        loop_invariants: BTreeMap::new(),
        loop_decreases: BTreeMap::new(),
    };
    let ret_local = builder.new_local(f.ret, None);
    debug_assert_eq!(ret_local, RET_LOCAL);
    let entry = builder.new_block();
    builder.current = entry;

    let mut params = Vec::new();
    for p in &f.params {
        let local = builder.new_local(p.ty, Some(p.name.clone()));
        builder.vars.insert(p.name.clone(), local);
        params.push(local);
    }

    builder.lower_block(&f.body)?;
    // If control can fall off the end of the body without an explicit
    // `return`, the current block has no terminator yet; a verifier
    // consuming this IR will find `ret_local` unassigned on that path
    // and can report it -- this crate doesn't guess a default.
    if builder.blocks[builder.current.0 as usize].terminator == Terminator::Unreachable {
        builder.set_terminator(builder.current, Terminator::Return);
    }

    Ok(Function {
        name: f.name.clone(),
        params,
        ret_local,
        locals: builder.locals,
        blocks: builder.blocks,
        requires: f.requires.clone(),
        ensures: f.ensures.clone(),
        loop_invariants: builder.loop_invariants,
        loop_decreases: builder.loop_decreases,
    })
}

/// Lower a full parsed program into MIR, one function at a time.
pub fn lower_program(program: &AstProgram) -> Result<Program, LowerError> {
    let mut seen = std::collections::HashSet::new();
    for f in &program.functions {
        if !seen.insert(f.name.clone()) {
            return Err(LowerError::DuplicateFunction(f.name.clone()));
        }
    }
    let mut functions = Vec::new();
    for f in &program.functions {
        functions.push(lower_function(f, &seen)?);
    }
    Ok(Program { functions })
}
