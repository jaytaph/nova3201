; hello_hi.s
; Prints "HI\n" to the UART and halts.

    .text
    .org 0x0

start:
        ; r1 = UART_TX = 0x8000_2200
        lui     r1, 0x8000      ; r1 = 0x8000_0000
        ori     r1, r1, 0x2200  ; r1 = 0x8000_2200

        ; 'H'
        addi    r2, r0, 72      ; 'H'
        sb      r2, 0(r1)

        ; 'I'
        addi    r2, r0, 73      ; 'I'
        sb      r2, 0(r1)

        ; '\n'
        addi    r2, r0, 10      ; newline
        sb      r2, 0(r1)

        halt
