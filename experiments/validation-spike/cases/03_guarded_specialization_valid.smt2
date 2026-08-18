; SPL-ARCH Stage-0 fixture 03
; semantic purpose : a specialization that drops the unsigned overflow check of
;                    checked_add_u8(x, 100) is correct only under the guard
;                    Gv(x) = x <u 156.
;
;   K(x):  if unsigned_overflow(x + 100) -> SemanticError(Overflow)
;          else                          -> Return(wrapping_add(x, 100))
;   P(x):  Return(wrapping_add(x, 100))          ; check elided
;   Gv(x): bvult x #x9c
;
; overflow rule    : unsigned overflow at width 8 iff the width-16 zero-extended
;                    sum differs from the zero extension of the width-8 sum.
; VC shape         : exists x. Gv(x) and K(x) != P(x)
; expect           : unsat
(set-logic QF_BV)

(declare-const x (_ BitVec 8))

(declare-const s (_ BitVec 8))
(assert (= s (bvadd x #x64)))
(declare-const ovf Bool)
(assert (= ovf (not (= ((_ zero_extend 8) s)
                       (bvadd ((_ zero_extend 8) x) ((_ zero_extend 8) #x64))))))

; K outcome
(declare-const k_tag (_ BitVec 1))
(declare-const k_val (_ BitVec 8))
(assert (= k_tag (ite ovf #b1 #b0)))
(assert (= k_val (ite ovf #x00 s)))

; P outcome
(declare-const p_tag (_ BitVec 1))
(declare-const p_val (_ BitVec 8))
(assert (= p_tag #b0))
(assert (= p_val s))

; guard Gv
(assert (bvult x #x9c))

; VC: the two outcomes disagree inside the guard
(assert (not (and (= k_tag p_tag) (= k_val p_val))))

(check-sat)
