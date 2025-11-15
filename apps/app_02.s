; spinner.s
; Prints a spinner sequence: |/-\|/-\... forever.

    .text
    .org 0x0

start:
        ; r1 = UART_TX = 0x8000_2200
        lui     r1, 0x8000      ; r1 = 0x8000_0000
        ori     r1, r1, 0x2200  ; r1 = 0x8000_2200


        ; Preload spinner characters into registers
        addi    r2, r0, 124     ; '|'  (ASCII 124)
        addi    r3, r0, 47      ; '/'  (ASCII 47)
        addi    r4, r0, 45      ; '-'  (ASCII 45)
        addi    r5, r0, 92      ; '\'  (ASCII 92)

loop:
        sb      r2, 0(r1)       ; '|'
        sb      r3, 0(r1)       ; '/'
        sb      r4, 0(r1)       ; '-'
        sb      r5, 0(r1)       ; '\'

        j       loop
