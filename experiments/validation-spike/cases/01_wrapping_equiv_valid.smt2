; SPL-ARCH Stage-0 fixture 01
; semantic purpose : wrapping-arithmetic equivalence of two structurally
;                    different u8 realizations of the same K expression
;                    K(x,y) = wrapping_add(wrapping_mul(x,3), y)
;                    P(x,y) = wrapping_add(wrapping_add(wrapping_add(x,x),x), y)
; VC shape         : exists x,y. K(x,y) != P(x,y)
; expect           : unsat
(set-logic QF_BV)

(declare-const x (_ BitVec 8))
(declare-const y (_ BitVec 8))

(assert (not (= (bvadd (bvmul x #x03) y)
                (bvadd (bvadd (bvadd x x) x) y))))

(check-sat)
