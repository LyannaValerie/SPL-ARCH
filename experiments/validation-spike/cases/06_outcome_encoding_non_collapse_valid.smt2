; SPL-ARCH Stage-0 fixture 06
; semantic purpose : the chosen outcome encoding cannot conflate a Return with a
;                    SemanticError, for any payload whatsoever. This is the
;                    positive counterpart of fixture 05 and is the property the
;                    future ObservedOutcome encoding must preserve.
; VC shape         : exists tag/payload assignment with tags Err and Return that
;                    the outcome-equality relation nevertheless accepts
; expect           : unsat
(set-logic QF_BV)

(declare-const k_tag (_ BitVec 1))
(declare-const k_val (_ BitVec 8))
(declare-const p_tag (_ BitVec 1))
(declare-const p_val (_ BitVec 8))

; K is a SemanticError, P is a Return; payloads are unconstrained
(assert (= k_tag #b1))
(assert (= p_tag #b0))

; claim to refute: outcome equality still holds
(assert (and (= k_tag p_tag) (= k_val p_val)))

(check-sat)
