; count_0_to_9.s
; Prints "0123456789" once and halts.

    .text
    .org 0x0000

start:
        ; r1 = UART_TX = 0x8000_2200
        lui     r1, 0x8000      ; r1 = 0x8000_0000
        ori     r1, r1, 0x2200  ; r1 = 0x8000_2200


        ; r2 = current character ('0')
        addi    r2, r0, 48      ; '0'

        ; r3 = limit ('9' + 1 = ':')
        addi    r3, r0, 58      ; ':' = 57 + 1

loop:
        sb      r2, 0(r1)       ; putchar(r2)

        addi    r2, r2, 1       ; r2++
        blt     r2, r3, loop    ; while (r2 < r3) goto loop

        ; newline for nice formatting
        addi    r2, r0, 10
        sb      r2, 0(r1)

        halt
