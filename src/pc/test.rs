use super::*;
use fnv::FnvHashMap;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
struct Var(usize);

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
enum Risc {
    /// var := constant
    Load(Var, isize),

    /// dst := src1 + src2
    Add(Var, Var, Var),

    /// dst := if src1 < src2 { 1 } else { 0 }
    Lt(Var, Var, Var),

    /// goto l
    Goto(Label),

    /// if src == 0 { goto l }
    JumpZ(Var, Label),

    /// do nothing
    NoOp,

    /// effectively, 'halt'
    Ret,
}

impl Instruction for Risc {
    fn successors(&self) -> Successors {
        match self {
            Risc::Load(_, _) => Successors::fallthrough(),
            Risc::Add(_, _, _) => Successors::fallthrough(),
            Risc::Lt(_, _, _) => Successors::fallthrough(),
            Risc::Goto(l) => Successors::goto(vec![*l]),
            Risc::JumpZ(_, l) => Successors::conditional(vec![*l]),
            Risc::NoOp => Successors::fallthrough(),
            Risc::Ret => Successors::halt(),
        }
    }
}

#[derive(Clone, Debug)]
struct ConstFact {
    // not in the map = bottom
    // in the map but None = top
    vars: FnvHashMap<Var, Option<isize>>,
}

impl ConstFact {
    fn pair(var: Var, constant: Option<isize>) -> Self {
        let mut vars = FnvHashMap::default();
        vars.insert(var, constant);
        ConstFact { vars }
    }

    fn lift<F>(&self, dst: Var, src1: Var, src2: Var, f: F) -> Self
    where
        F: FnOnce(isize, isize) -> isize,
    {
        match (self.vars.get(&src1), self.vars.get(&src2)) {
            (Some(Some(v1)), Some(Some(v2))) => Self::pair(dst, Some(f(*v1, *v2))),
            (Some(None), Some(None)) => Self::bottom(),
            _ => Self::pair(dst, None),
        }
    }

    fn merge(&self, other: &Self) -> Self {
        let mut vars = self.vars.clone();
        for (k, v) in &other.vars {
            vars.insert(*k, *v);
        }
        ConstFact { vars }
    }
}

impl Lattice for ConstFact {
    fn bottom() -> Self {
        ConstFact {
            vars: FnvHashMap::default(),
        }
    }

    fn top() -> Self {
        // this is not strictly speaking, correct
        Self::bottom()
    }

    fn join(&mut self, other: &Self, label: Label) -> bool {
        let mut changed = false;

        for (key, val) in &other.vars {
            match self.vars.get(&key) {
                Some(Some(c)) => {
                    // I have a value, so if we're not the same, we become top.
                    if Some(*c) != *val {
                        changed = true;
                        self.vars.insert(*key, None);
                    }
                }
                Some(None) => {
                    // I'm already top, so no change is possible.
                    ;
                }
                None => {
                    // I'm bottom, so I'll be whatever (non-bottom) value you have.
                    changed = true;
                    self.vars.insert(*key, *val);
                }
            }
        }
        dbg!(changed);
        changed
    }
}

#[derive(Clone)]
struct ConstAnalysis;

impl Analysis<Risc, ConstFact> for ConstAnalysis {
    fn analyze(
        &mut self,
        _graph: &Graph<Risc>,
        _label: Label,
        instruction: &Risc,
        fact: &ConstFact,
    ) -> Rewrite<Risc, ConstFact> {
        match instruction {
            Risc::Load(var, constant) => Rewrite::Fact(ConstFact::pair(*var, Some(*constant))),
            Risc::Add(dst, src1, src2) => {
                Rewrite::Fact(fact.merge(&fact.lift(*dst, *src1, *src2, |a, b| a + b)))
            }
            Risc::Lt(dst, src1, src2) => Rewrite::Fact(fact.merge(&fact.lift(
                *dst,
                *src1,
                *src2,
                |a, b| if a < b { 1 } else { 0 },
            ))),
            Risc::Goto(_l) => Rewrite::NoChange,
            Risc::JumpZ(_src, _l) => Rewrite::NoChange,
            Risc::NoOp => Rewrite::NoChange,
            Risc::Ret => Rewrite::NoChange,
        }
    }
}

#[test]
fn constant_prop() {
    // TODO: We could use a builder to help with label issue.
    //  perhaps instructions could take a label type... symbolic vs literal
    let code = vec![
        /* 00 */ Risc::Load(Var(0), 0),
        /* 01 */ Risc::Load(Var(1), 1),
        /* 02 */ Risc::Load(Var(2), 10),
        /* 03 */ Risc::Lt(Var(3), Var(0), Var(2)),
        /* 04 */ Risc::JumpZ(Var(3), Label::new(0, 0x08)),
        /* 05 */ Risc::Add(Var(4), Var(1), Var(1)),
        /* 06 */ Risc::Add(Var(0), Var(0), Var(4)),
        /* 07 */ Risc::Goto(Label::new(0, 0x03)),
        /* 08 */ Risc::Ret,
    ];

    let graph = Graph::new(code);
    let mut analysis = ConstAnalysis;

    let fact_base = forward_analyze(&mut analysis, &graph);

    println!("{:?}", fact_base);
}
