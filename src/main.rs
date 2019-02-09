mod dataflow;

mod test {
    use crate::dataflow::*;
    use std::collections::HashMap;


    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    struct Var(u16);

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    struct Constant(usize);

    #[derive(Copy, Clone, Debug, Hash)]
    enum Arith {
        Add,
        Sub,
        And,
        Or,
    }

    #[derive(Copy, Clone, Debug, Hash)]
    enum Cond {
        EQ,
        NEQ,
        LT,
        LTE,
    }

    #[derive(Copy, Clone, Debug, Hash)]
    enum RISC {
        Label(Label),
        Load(Var, Constant),
        Arith(Arith, Var, Var, Var),
        Cond(Cond, Var, Var, Label, Label),
        Jump(Label),
        Ret,
    }

    impl Instruction for RISC {
        fn label(&self) -> Option<Label> {
            match self {
                RISC::Label(l) => Some(*l),
                _ => None
            }
        }

        fn successors(&self) -> Option<Vec<Label>> {
            match self {
                RISC::Cond(_, _, _, l1, l2) => Some(vec![*l1, *l2]),
                RISC::Jump(l) => Some(vec![*l]),
                RISC::Ret => Some(vec![]),
                _ => None
            }
        }
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    enum WithTop<T> {
        Top,
        Elem(T),
    }

    #[derive(Clone, Debug)]
    struct ConstFact {
        vars: HashMap<Var, WithTop<Constant>>
    }

    impl ConstFact {
        fn new() -> ConstFact {
            ConstFact { vars: HashMap::new() }
        }

        fn set(&mut self, var: Var, constant: Constant) {
            self.vars.insert(var, WithTop::Elem(constant));
        }

        fn get(&self, var: Var) -> Option<WithTop<Constant>> {
            self.vars.get(&var).cloned()
        }

        fn get_const(&self, var: Var) -> Option<Constant> {
            match self.vars.get(&var) {
                Some(WithTop::Elem(c)) => Some(*c),
                _ => None
            }
        }
    }

    impl Lattice for ConstFact {
        fn bottom() -> Self {
            ConstFact::new()
        }

        fn join(&mut self, other: &Self, _label: Label) -> bool {
            let mut changed = false;
            for (k, v) in &other.vars {
                let old = self.get(*k);
                let new = match (old, v) {
                    (Some(WithTop::Top), _) => WithTop::Top,
                    (_, WithTop::Top) => WithTop::Top,
                    (Some(WithTop::Elem(x)), WithTop::Elem(y)) =>
                        if x == *y { WithTop::Elem(x) } else { WithTop::Top },
                    (None, anything) => *anything
                };

                if old != Some(new) {
                    changed = true;
                    self.vars.insert(*k, new);
                }
            }

            changed
        }
    }

    struct ConstantPropagation;

    impl ForwardAnalysis<RISC, ConstFact> for ConstantPropagation {
        fn analyze(&mut self,
                   _graph: &Graph<RISC>,
                   _label: Label,
                   instruction: &RISC,
                   analyze: Analyze<ConstFact>)
                   -> Option<Rewrite<RISC>> {
            match instruction {
                RISC::Label(_label) => {
                    None
                }
                RISC::Load(var, constant) => {
                    analyze.fact_mut().set(*var, *constant);
                    None
                }
                RISC::Arith(arith, dst, src1, src2) => {
                    let facts = analyze.fact();

                    if let (Some(Constant(c1)), Some(Constant(c2))) =
                    (facts.get_const(*src1), facts.get_const(*src2)) {
                        let result = match arith {
                            Arith::Add => c1 + c2,
                            Arith::Sub => c1 - c2,
                            Arith::And => c1 & c2,
                            Arith::Or => c1 | c2
                        };

                        return Some(analyze.replace_single(RISC::Load(*dst, Constant(result))));
                    }
                    None
                }
                RISC::Cond(_cond, _src1, _src2, _l1, _l2) => {
                    // For now...
                    None
                }
                RISC::Jump(_l1) => {
                    None
                }
                RISC::Ret => {
                    None
                }
            }
        }
    }

    #[test]
    fn analysis_test() {
        let entry = Label(0);
        let loop_body = Label(1);
        let exit = Label(2);
        let block0 = Block::new(vec![
            RISC::Label(entry),
            RISC::Load(Var(0), Constant(0)),
            RISC::Load(Var(1), Constant(1)),
            RISC::Load(Var(2), Constant(5)),
            RISC::Jump(loop_body)
        ]).unwrap();

        let block1 = Block::new(vec![
            RISC::Label(loop_body),
            RISC::Arith(Arith::Sub, Var(2), Var(2), Var(1)),
            RISC::Cond(Cond::EQ, Var(2), Var(0), exit, loop_body)
        ]).unwrap();

        let block2 = Block::new(vec![
            RISC::Label(exit),
            RISC::Ret
        ]).unwrap();

        let graph = Graph::from_blocks(vec![block0, block1, block2]);

        // Strictly speaking, we'd want an entry fact that had all vars as Top..
        let entry_fact = ConstFact::bottom();
        let mut analysis = ConstantPropagation;

        let fact_base = forward_analysis(&mut analysis, &graph, entry, entry_fact);
        println!("{:?}", fact_base);
    }
}

fn main() {
    println!("Hello, world!");
}
