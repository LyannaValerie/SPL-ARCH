; SPL-ARCH Stage-0 fixture 07
; semantic purpose : validate the signed-overflow rule used by fixture 02 instead
;                    of trusting it. The width-extension rule
;                        sext16(a +8 b) != sext16(a) +16 sext16(b)
;                    is proved equivalent, over all width-8 inputs, to the
;                    classical sign-case rule
;                        (a>=0 and b>=0 and a+b<0) or (a<0 and b<0 and a+b>=0)
;                    Boundary inputs such as 1 + 127 and -1 + -128 are therefore
;                    covered by the proof rather than by spot checks.
; VC shape         : exists a,b. rule_extension(a,b) != rule_sign_case(a,b)
; expect           : unsat
(set-logic QF_BV)

(declare-const a (_ BitVec 8))
(declare-const b (_ BitVec 8))

(declare-const rule_extension Bool)
(assert (= rule_extension
           (not (= ((_ sign_extend 8) (bvadd a b))
                   (bvadd ((_ sign_extend 8) a) ((_ sign_extend 8) b))))))

(declare-const rule_sign_case Bool)
(assert (= rule_sign_case
           (or (and (bvsge a #x00) (bvsge b #x00) (bvslt (bvadd a b) #x00))
               (and (bvslt a #x00) (bvslt b #x00) (bvsge (bvadd a b) #x00)))))

(assert (not (= rule_extension rule_sign_case)))

(check-sat)
