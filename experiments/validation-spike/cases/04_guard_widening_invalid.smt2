; SPL-ARCH Stage-0 fixture 04
; semantic purpose : fixture 03 with the guard widened from x <u 156 to x <u 255.
;                    The specialization is no longer sound: inside the widened
;                    guard there are inputs where K raises SemanticError(Overflow)
;                    while P returns a wrapped value.
; VC shape         : exists x. Gv_wide(x) and K(x) != P(x)
; expect           : sat        (x = 200 is a witness)
(set-logic QF_BV)

(declare-const x (_ BitVec 8))

(declare-const s (_ BitVec 8))
(assert (= s (bvadd x #x64)))
(declare-const ovf Bool)
(assert (= ovf (not (= ((_ zero_extend 8) s)
                       (bvadd ((_ zero_extend 8) x) ((_ zero_extend 8) #x64))))))

(declare-const k_tag (_ BitVec 1))
(declare-const k_val (_ BitVec 8))
(assert (= k_tag (ite ovf #b1 #b0)))
(assert (= k_val (ite ovf #x00 s)))

(declare-const p_tag (_ BitVec 1))
(declare-const p_val (_ BitVec 8))
(assert (= p_tag #b0))
(assert (= p_val s))

; widened guard Gv
(assert (bvult x #xff))

(assert (not (and (= k_tag p_tag) (= k_val p_val))))

(check-sat)
