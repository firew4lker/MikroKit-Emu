; HD44780 LCD (16x2 or 20x4) on the 8155 port B, 4-bit mode.
; Connect the LCD with the LCD menu, load this file, then press GO and EX
; (or G at the terminal).
;
;   PB2 = RS, PB3 = E, PB4-PB7 = D4-D7 (X2 pins 15, 16, 13, 14, 11, 12)
;   D0-D3 and R/W are tied to ground, so the program can't read the busy
;   flag and waits instead: about 50 us per byte and 2 ms after Clear.

CSR     EQU 40H         ; 8155 command/status register
PORTB   EQU 42H
USCSR   EQU 40FFH       ; the monitor wants a copy of the CSR here
RS      EQU 04H
E       EQU 08H

        ORG 2000H
START:  MVI A,02H       ; port B = output, A and C = inputs
        OUT CSR
        STA USCSR
        XRA A
        OUT PORTB
        LXI B,6000      ; > 40 ms after power-on
        CALL DELAY
; the reset-by-instruction sequence: 3, 3, 3 in 8-bit mode, then 2
        MVI A,30H
        CALL INIT_NIB
        LXI B,600       ; > 4.1 ms
        CALL DELAY
        MVI A,30H
        CALL INIT_NIB
        LXI B,20        ; > 100 us
        CALL DELAY
        MVI A,30H
        CALL INIT_NIB
        LXI B,20
        CALL DELAY
        MVI A,20H       ; now 4-bit mode
        CALL INIT_NIB
        LXI B,20
        CALL DELAY
        MVI A,28H       ; 4-bit, 2 lines, 5x8 dots
        CALL CMD
        MVI A,0CH       ; display on, cursor off
        CALL CMD
        MVI A,01H       ; clear
        CALL CMD
        LXI B,250       ; Clear takes 1.52 ms
        CALL DELAY
        MVI A,06H       ; cursor moves right, no display shift
        CALL CMD

        MVI A,80H       ; row 1 (DDRAM 00H)
        LXI H,LINE1
        CALL ROW
        MVI A,0C0H      ; row 2 (40H)
        LXI H,LINE2
        CALL ROW
        MVI A,94H       ; row 3 of a 20x4 (14H); off screen on a 16x2
        LXI H,LINE3
        CALL ROW
        MVI A,0D4H      ; row 4 of a 20x4 (54H)
        LXI H,LINE4
        CALL ROW
        RST 1           ; back to the monitor

; set the DDRAM address in A, then write the string at HL (ends with 0)
ROW:    CALL CMD
PUTS:   MOV A,M
        ORA A
        RZ
        CALL CHR
        INX H
        JMP PUTS

; A = instruction (CMD) or character (CHR)
CMD:    MVI D,0
        JMP BYTE
CHR:    MVI D,RS
BYTE:   PUSH PSW
        ANI 0F0H        ; high nibble first
        CALL NIBBLE
        POP PSW
        RLC
        RLC
        RLC
        RLC
        ANI 0F0H        ; then the low nibble
        CALL NIBBLE
        LXI B,6         ; ~50 us for the LCD to finish
        JMP DELAY

; one 4-bit transfer: A = data in bits 4-7, D = RS
INIT_NIB: MVI D,0
NIBBLE: ORA D
        OUT PORTB       ; data and RS first
        ORI E
        OUT PORTB       ; E high
        XRI E
        OUT PORTB       ; E low: the LCD latches the nibble
        RET

; wait BC x 7.8 us (24 clock cycles a loop at 3.072 MHz)
DELAY:  DCX B
        MOV A,B
        ORA C
        JNZ DELAY
        RET

LINE1:  DB "Mikrokit-Emu", 0
LINE2:  DB "8085 + HD44780", 0
LINE3:  DB "Row 3 of a 20x4", 0
LINE4:  DB "Row 4: 0123456789", 0
        END START
