# SPL-ARCH

SPL-ARCH investiga uma arquitetura/protocolo de confiança para **realização adaptativa** de uma computação.

A hipótese V1 é deliberadamente estreita: produtores de realizações e planners adaptativos podem permanecer fora do núcleo semântico confiável, desde que **nenhuma realização execute sem admissão obrigatória contra uma autoridade semântica persistente** e que o sistema preserve a identidade entre aquilo que foi provado, admitido, ativado e efetivamente executado.

## Estado atual

```text
Theoretical V1: FROZEN
Implementation Plan V1: FROZEN
Implementation: NOT STARTED
Next implementation gate: STAGE 0 — VALIDATION FEASIBILITY SPIKE
Universal originality: NOT CLAIMED
Prior-art subsumption: NOT ESTABLISHED
```

A V1 não é ainda uma linguagem de programação, IR, compilador ou runtime final. A primeira implementação planejada é **SPL-0**, um protótipo end-to-end restrito para testar honestamente o protocolo de confiança antes de ampliar semântica, targets ou hardware.

## Documentação V1

- [`docs/SPL-ARCH-V1.md`](docs/SPL-ARCH-V1.md) — especificação teórica congelada.
- [`docs/SPL-0-IMPLEMENTATION-PLAN-V1.md`](docs/SPL-0-IMPLEMENTATION-PLAN-V1.md) — plano de implementação SPL-0 congelado, do Stage 0 à primeira release end-to-end.
- [`docs/AGENT-C-STAGE0.md`](docs/AGENT-C-STAGE0.md) — prompt executável para o primeiro agente implementador, limitado ao Validation Feasibility Spike.
- [`docs/EXPERIMENTS-V1.md`](docs/EXPERIMENTS-V1.md) — plano experimental teórico e bateria adversarial.
- [`docs/PRIOR-ART.md`](docs/PRIOR-ART.md) — estado de prior art, não-reivindicações e demarcação provisória.

## Núcleo conceitual

A cadeia V1 é:

```text
Semantic Contract K
        +
Assurance Policy A
        ↓
Candidate
        ↓
VC = DeriveObligation(...)
        ↓
Validation Evidence V
        ↓
Admission
        ↓
Adaptive decision
        ↓
ActivationGate
        ↓
Admission-to-Execution Integrity
        ↓
Execute exact admitted realization
```

O objetivo não é demonstrar que SIMD, GPU ou FPGA são rápidos. É verificar se um **backend futuro e não confiável** consegue produzir uma realização correta que seja aceita e uma realização incorreta que seja rejeitada, sem recuperar estado semântico oculto obrigatório do frontend e sem adquirir autoridade no Runtime Semantic TCB.

## Caminho de implementação congelado

```text
Stage 0  Validation Feasibility Spike
Stage 1  Semantic Vertical Slice
Stage 2  Trust Vertical Slice
Stage 3  Minimal SPL Frontend
Stage 4  Reference End-to-End
Stage 5  Official T0 Freeze
Stage 6  Future Untrusted Backend B
Stage 7  Adaptive Planner
Stage 8  SPL-0 Release
```

O primeiro trabalho técnico é deliberadamente menor que a linguagem: testar se obrigações `QF_BV` representativas conseguem produzir evidência reproduzível, vinculada ao problema exato e verificável independentemente através de `CPC/Ethos` ou `Alethe/Carcara`. Se ambos falharem, o plano exige registrar a falha e usar explicitamente o fallback solver-in-TCB, em vez de fingir uma garantia que o experimento não obteve.

Veja a especificação e o plano antes de tratar qualquer hipótese como implementação confirmada. A humanidade já inventou abstrações suficientes; não precisamos também inventar resultados experimentais.
