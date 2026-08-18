; SPL-ARCH Stage-0 fixture 10
; semantic purpose : strength reduction of a signed division to an arithmetic
;                    shift, valid only under the guard 0 <=s x <s 64. This case
;                    exercises signed comparisons together with bvsdiv/bvashr,
;                    which are handled by different cvc5 rewrite families than
;                    the additive cases above.
;
;   K(x):  Return(bvsdiv x 2)
;   P(x):  Return(bvashr x 1)
;   Gv(x): (bvsge x 0) and (bvslt x 64)
;
; VC shape         : exists x. Gv(x) and K(x) != P(x)
; expect           : unsat
(set-logic QF_BV)

(declare-const x (_ BitVec 8))

(declare-const k_val (_ BitVec 8))
(assert (= k_val (bvsdiv x #x02)))

(declare-const p_val (_ BitVec 8))
(assert (= p_val (bvashr x #x01)))

; guard Gv
(assert (bvsge x #x00))
(assert (bvslt x #x40))

(assert (not (= k_val p_val)))

(check-sat)
