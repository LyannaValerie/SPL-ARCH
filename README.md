# SPL-ARCH

SPL-ARCH investiga uma arquitetura/protocolo de confiança para **realização adaptativa** de uma computação.

A hipótese V1 é deliberadamente estreita: produtores de realizações e planners adaptativos podem permanecer fora do núcleo semântico confiável, desde que **nenhuma realização execute sem admissão obrigatória contra uma autoridade semântica persistente** e que o sistema preserve a identidade entre aquilo que foi provado, admitido, ativado e efetivamente executado.

## Estado atual

```text
Theoretical V1: FROZEN
Implementation: NOT STARTED
Experimental phase: NEXT
Universal originality: NOT CLAIMED
Prior-art subsumption: NOT ESTABLISHED
```

A V1 não é ainda uma linguagem de programação, IR, compilador ou runtime final. A possibilidade de uma futura **Smart Programming Language (SPL)** fica condicionada à evidência experimental produzida pela arquitetura.

## Documentação V1

- [`docs/SPL-ARCH-V1.md`](docs/SPL-ARCH-V1.md) — especificação teórica congelada.
- [`docs/EXPERIMENTS-V1.md`](docs/EXPERIMENTS-V1.md) — plano experimental SPL-0 e bateria adversarial.
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

O primeiro objetivo experimental não é demonstrar que SIMD, GPU ou FPGA são rápidos. É verificar se um **backend futuro e não confiável** consegue produzir uma realização correta que seja aceita e uma realização incorreta que seja rejeitada, sem recuperar estado semântico oculto do frontend e sem entrar no Semantic TCB.

## Próxima fase

A formalização teórica normal foi encerrada após duas rodadas adversariais consecutivas estruturalmente estáveis e uma auditoria final de congelamento.

O trabalho agora muda de natureza:

```text
Frozen V1
    ↓
SPL-0 Admission Kernel
    ↓
adversarial tests
    ↓
future untrusted backend
    ↓
adaptive realization population
    ↓
architectural adaptation experiment
```

Veja a especificação antes de tratar qualquer hipótese como implementação confirmada. A humanidade já inventou coisas demais para que o projeto possa se dar ao luxo de confundir proposta com fato.
