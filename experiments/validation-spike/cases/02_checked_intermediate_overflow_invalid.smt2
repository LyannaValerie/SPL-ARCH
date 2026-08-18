; SPL-ARCH Stage-0 fixture 02
; semantic purpose : an intermediate checked_add can raise SemanticError(Overflow);
;                    a candidate that algebraically cancels (+MAX) then (-MAX)
;                    and returns x loses that intermediate outcome.
;
;   K(x):  t = checked_add_i8(x, 127)
;          if overflow(t)          -> SemanticError(Overflow)
;          u = checked_sub_i8(t, 127)
;          if overflow(u)          -> SemanticError(Overflow)
;          else                    -> Return(u)
;
;   P(x):  Return(x)
;
; outcome encoding : (tag, payload); tag #b0 = Return, tag #b1 = SemanticError(Overflow);
;                    payload is canonically #x00 whenever tag = #b1.
; overflow rule    : signed overflow of a op b at width 8 iff the width-16
;                    sign-extended result differs from the sign extension of the
;                    width-8 result. Fixture 07 proves this rule agrees with the
;                    classical sign-case rule.
; VC shape         : exists x. K(x) != P(x)
; expect           : sat        (x = 1 is a witness: K = Err(Overflow), P = Return(1))
(set-logic QF_BV)

(declare-const x (_ BitVec 8))

; t = wrapping result of x + 127, ovf1 = signed overflow of that addition
(declare-const t (_ BitVec 8))
(assert (= t (bvadd x #x7f)))
(declare-const ovf1 Bool)
(assert (= ovf1 (not (= ((_ sign_extend 8) t)
                        (bvadd ((_ sign_extend 8) x) ((_ sign_extend 8) #x7f))))))

; u = wrapping result of t - 127, ovf2 = signed overflow of that subtraction
(declare-const u (_ BitVec 8))
(assert (= u (bvsub t #x7f)))
(declare-const ovf2 Bool)
(assert (= ovf2 (not (= ((_ sign_extend 8) u)
                        (bvsub ((_ sign_extend 8) t) ((_ sign_extend 8) #x7f))))))

; K outcome
(declare-const k_tag (_ BitVec 1))
(declare-const k_val (_ BitVec 8))
(assert (= k_tag (ite ovf1 #b1 (ite ovf2 #b1 #b0))))
(assert (= k_val (ite ovf1 #x00 (ite ovf2 #x00 u))))

; P outcome
(declare-const p_tag (_ BitVec 1))
(declare-const p_val (_ BitVec 8))
(assert (= p_tag #b0))
(assert (= p_val x))

; VC: the two outcomes disagree
(assert (not (and (= k_tag p_tag) (= k_val p_val))))

(check-sat)
