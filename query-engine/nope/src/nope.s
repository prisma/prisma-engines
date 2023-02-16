    ;; We have to use a separately compiled assembly module and can't
    ;; use the std::arch::asm! macro in Rust because we want to avoid
    ;; LLVM messing with our beautiful nops, reduce build time, and
    ;; prevent the compiler from exploding when using a big enough
    ;; value of NNOPS.

    ;; Polyglot program! Valid as both x86_64 and aarch64 assembly
    ;; code.  You only need to tweak the NNOPS variable to match the
    ;; desired code size: one nop instruction is 1 byte on x86_64 and
    ;; 4 bytes on aarch64.

    .text

    .set NNOPS, 20000000
    .globl _nope_nops

_nope_nops:
    .rept NNOPS
    nop
    .endr
    ret

    ;; the macro doesn't work in llvm assembler, only in gnu assembler:
    ;; <instantiation>:6:16: error: expected absolute expression
    ;;         gen_fn %from+1, 3
    ;;                ^
    ;; src/nope.s:26:5: note: while in macro instantiation
    ;;     gen_fn 0, 3

    ;; .altmacro

    ;; .macro gen_fn from, to
    ;; .globl _nope_fn_\from
    ;; _nope_fn_\from:
    ;;     .rept NNOPS
    ;;     nop
    ;;     .endr
    ;;     ret
    ;; .if \from-\to
    ;;     gen_fn %from+1, \to
    ;; .endif
    ;; .endm

    ;; gen_fn 0, 3

