; SPL-ARCH Stage-0 fixture 08
; semantic purpose : modelling checked_add as plain wrapping bvadd is unsound.
;                    Without any guard, the unchecked realization disagrees with
;                    the checked contract exactly on the overflowing inputs.
;
;   K(x,y): if unsigned_overflow(x + y) -> SemanticError(Overflow)
;           else                        -> Return(wrapping_add(x, y))
;   P(x,y): Return(wrapping_add(x, y))
;
; VC shape         : exists x,y. K(x,y) != P(x,y)
; expect           : sat
(set-logic QF_BV)

(declare-const x (_ BitVec 8))
(declare-const y (_ BitVec 8))

(declare-const s (_ BitVec 8))
(assert (= s (bvadd x y)))
(declare-const ovf Bool)
(assert (= ovf (not (= ((_ zero_extend 8) s)
                       (bvadd ((_ zero_extend 8) x) ((_ zero_extend 8) y))))))

(declare-const k_tag (_ BitVec 1))
(declare-const k_val (_ BitVec 8))
(assert (= k_tag (ite ovf #b1 #b0)))
(assert (= k_val (ite ovf #x00 s)))

(declare-const p_tag (_ BitVec 1))
(declare-const p_val (_ BitVec 8))
(assert (= p_tag #b0))
(assert (= p_val s))

(assert (not (and (= k_tag p_tag) (= k_val p_val))))

(check-sat)
