//! This module provides support for removing the extraneous break statements
//! generated by the incremental relooper.
use super::*;

pub struct IncCleanup {
    in_tail: Option<ImplicitReturnType>,
    brk_lbl: Label,
}

impl IncCleanup {
    pub fn new(in_tail: Option<ImplicitReturnType>, brk_lbl: Label) -> Self {
        IncCleanup{in_tail, brk_lbl}
    }

    /// The only way we can say for sure that we don't need a labelled block is if we remove
    /// the (unique) break to that label. We know that the label will be unique because relooper
    /// never duplicates blocks.
    ///
    /// Returns true if we manage to remove a tail expr.
    pub fn remove_tail_expr(&self, stmts: &mut Vec<Stmt>) -> bool {
        if let Some(mut stmt) = stmts.pop() {
            // If the very last stmt in our relooped output is a return/break, we can just
            // remove that statement. We additionally know that there is definitely no need
            // to label a block (if we were in that mode in the first place).
            if self.is_idempotent_tail_expr(&stmt) {
                return true;
            }

            let mut removed_tail_expr = false;

            if let StmtKind::Expr(ref mut expr) = stmt.node {
                match expr.node {
                    ExprKind::If(_, ref mut body, ref mut sels) => {
                        removed_tail_expr = removed_tail_expr || self.remove_tail_expr(&mut body.stmts);
                        if let Some(els) = sels {
                            if let ExprKind::Block(ref mut blk, _) = els.node {
                                removed_tail_expr = removed_tail_expr || self.remove_tail_expr(&mut blk.stmts)
                            }
                        }
                    }

                    ExprKind::Match(_, ref mut cases) => {
                        // Block label can be removed from any arm
                        for case in cases {
                            match case.body.node {
                                ExprKind::Block(ref mut blk, _) => {
                                    removed_tail_expr = removed_tail_expr || self.remove_tail_expr(&mut blk.stmts)
                                }
                                _ => (),
                            }
                        }
                    }

                    _ => (),
                }
            }

            stmt = cleanup_if(stmt);

            // In all other cases, we give up and accept that we can't get rid of the last
            // stmt and that we might need a block label.
            stmts.push(stmt);
            removed_tail_expr
        } else {
            false
        }
    }

    fn is_idempotent_tail_expr(&self, stmt: &Stmt) -> bool {
        let tail_expr = if let Stmt { node: StmtKind::Semi(ref expr), .. } = *stmt {
            expr
        } else {
            return false
        };
        match self.in_tail {
            Some(ImplicitReturnType::Main) => {
                if let Expr { node: ExprKind::Ret(Some(ref zero)), .. } = **tail_expr {
                    if let Expr { node: ExprKind::Lit(ref lit), .. } = **zero {
                        if let Lit { node: LitKind::Int(0, LitIntType::Unsuffixed), .. } = *lit {
                            return true;
                        }
                    }
                }
                false
            }

            Some(ImplicitReturnType::Void) => {
                if let Expr { node: ExprKind::Ret(None), .. } = **tail_expr {
                    return true;
                }
                false
            }

            _ => {
                if let Expr { node: ExprKind::Break(Some(ref blbl), None), .. } = **tail_expr {
                    if blbl.ident == mk().label(self.brk_lbl.pretty_print()).ident {
                        return true;
                    }
                }
                false
            }

        }
    }
}

/// Remove empty else clauses from if expressions that can arise from
/// removing idempotent statements.
fn cleanup_if(stmt: Stmt) -> Stmt {
    if let Stmt { node: StmtKind::Expr(ref expr), .. } = &stmt {
        if let Expr { node: ExprKind::If(ref cond, ref body, ref els), .. } = **expr {
            if let Some(ref els) = els {
                if let Expr { node: ExprKind::Block(ref blk, None), .. } = **els {
                    if blk.stmts.is_empty() {
                        return Stmt {
                            node: StmtKind::Expr(P(Expr {
                                node: ExprKind::If(cond.clone(), body.clone(), None),
                                ..(**expr).clone()
                            })),
                            ..stmt
                        }
                    }
                }
            }
        }
    }
    stmt
}