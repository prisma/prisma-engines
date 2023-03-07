/*
   We have to use a separately compiled assembly module and can't use
   the std::arch::asm! macro in Rust because we want to avoid LLVM
   messing with our beautiful nops, reduce build time, and prevent the
   compiler from exploding when using a big enough value of N_NOPS.

   You need to tweak the N_NOPS constant to match the desired code
   size: one nop instruction is 1 byte on x86_64 and 4 bytes on
   aarch64.
*/

#ifdef __APPLE__
#define SYM(X) _##X
#else
#define SYM(X) X
#endif

/*
 * If this is defined, the implementation with simple nops block is used.
 * N_NOPS constant below controls the number of nop instructions.
 *
 * Otherwise, a different implementation that attempts to stress-test the
 * relocations in the dynamic linker is used.  N_CALLS constant controls the
 * number of blocks of calls to glibc symbols, and N_NOPS_PER_CALL constant
 * controls the number of nop instructions between each call.
*/
#define SIMPLE_NOPS

    .text

    .set    N_NOPS, 10000000
    .set    N_CALL_GROUPS, 1
    .set    N_NOPS_PER_CALL, 300
    .globl  SYM(nope_nops)

#ifdef SIMPLE_NOPS

SYM(nope_nops):
    .rept   N_NOPS
    nop
    .endr
    ret

#else

#include "libc_functions.inc"

#if defined(__aarch64__) || defined(_M_ARM64)
#define CALL(F)                         \
    stp     x29, x30, [sp, -16]! __NL__ \
    mov     x29, sp              __NL__ \
    bl      F                    __NL__ \
    ldp     x29, x30, [sp], 16   __NL__
#elif defined(__x86_64__) || defined(_M_X64)
#define CALL(F)                         \
    call    F                    __NL__ \
#else
#error can't detect x86_64 or aarch64
#endif

SYM(nope_nops):
    .rept   N_CALL_GROUPS
#define X(F)                       \
    CALL(SYM(F))            __NL__ \
    .rept   N_NOPS_PER_CALL __NL__ \
    nop                     __NL__ \
    .endr                   __NL__
LIBC_FUNCTIONS
#undef X
    .endr
    ret

#endif

    // the macro doesn't work in llvm assembler, only in gnu assembler:
    // <instantiation>:6:16: error: expected absolute expression
    //         gen_fn %from+1, 3
    //                ^
    // src/nope.s:26:5: note: while in macro instantiation
    //     gen_fn 0, 3
    //
    // maybe TODO: use a C preprocessor macro instead if there is
    // need in multiple functions

    // .altmacro

    // .macro gen_fn from, to
    // .globl _nope_fn_\from
    // _nope_fn_\from:
    //     .rept NNOPS
    //     nop
    //     .endr
    //     ret
    // .if \from-\to
    //     gen_fn %from+1, \to
    // .endif
    // .endm

    // gen_fn 0, 3

