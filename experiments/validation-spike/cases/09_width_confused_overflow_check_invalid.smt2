; SPL-ARCH Stage-0 fixture 09
; semantic purpose : a realization that performs the u8 contract in u32 and then
;                    truncates checks overflow at the wrong width. The 32-bit
;                    check never fires, so the u8 SemanticError(Overflow) is lost.
;
;   K(x,y): 8-bit checked_add   -> Err on u8 overflow, else Return(x +8 y)
;   P(x,y): z32 = zext32(x) +32 zext32(y)
;           if unsigned_overflow_32(z32) -> SemanticError(Overflow)   ; never true
;           else                         -> Return(extract 7..0 z32)
;
; VC shape         : exists x,y. K(x,y) != P(x,y)
; expect           : sat
(set-logic QF_BV)

(declare-const x (_ BitVec 8))
(declare-const y (_ BitVec 8))

; K: contract at width 8
(declare-const s8 (_ BitVec 8))
(assert (= s8 (bvadd x y)))
(declare-const ovf8 Bool)
(assert (= ovf8 (not (= ((_ zero_extend 8) s8)
                        (bvadd ((_ zero_extend 8) x) ((_ zero_extend 8) y))))))
(declare-const k_tag (_ BitVec 1))
(declare-const k_val (_ BitVec 8))
(assert (= k_tag (ite ovf8 #b1 #b0)))
(assert (= k_val (ite ovf8 #x00 s8)))

; P: realization at width 32, overflow checked at width 32
(declare-const z32 (_ BitVec 32))
(assert (= z32 (bvadd ((_ zero_extend 24) x) ((_ zero_extend 24) y))))
(declare-const ovf32 Bool)
(assert (= ovf32 (not (= ((_ zero_extend 32) z32)
                         (bvadd ((_ zero_extend 56) x) ((_ zero_extend 56) y))))))
(declare-const p_tag (_ BitVec 1))
(declare-const p_val (_ BitVec 8))
(assert (= p_tag (ite ovf32 #b1 #b0)))
(assert (= p_val (ite ovf32 #x00 ((_ extract 7 0) z32))))

(assert (not (and (= k_tag p_tag) (= k_val p_val))))

(check-sat)
