; SPL-ARCH Stage-0 fixture 05
; semantic purpose : the numeric payload of the two outcomes coincides while the
;                    semantic tag differs. A validation encoding that compared
;                    payloads only would wrongly report equivalence.
;
;   K = SemanticError(Overflow)   encoded (#b1, #x00)
;   P = Return(0)                 encoded (#b0, #x00)
;
; VC shape         : exists . K != P    with payloads asserted equal
; expect           : sat
(set-logic QF_BV)

(declare-const k_tag (_ BitVec 1))
(declare-const k_val (_ BitVec 8))
(declare-const p_tag (_ BitVec 1))
(declare-const p_val (_ BitVec 8))

; K = SemanticError(Overflow)
(assert (= k_tag #b1))
(assert (= k_val #x00))

; P = Return(0)
(assert (= p_tag #b0))
(assert (= p_val #x00))

; the payloads really do coincide
(assert (= k_val p_val))

; VC: the two outcomes disagree
(assert (not (and (= k_tag p_tag) (= k_val p_val))))

(check-sat)
