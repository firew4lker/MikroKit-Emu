; Counts 00..FF in the data field of the display using the monitor's
; UPDDT routine, then returns to the monitor with RST 1.
; Load it (File > Load program), then press GO and EX.

CURDT   EQU 40F6H       ; monitor's "current data" byte
UPDDT   EQU 03A9H       ; monitor routine: show CURDT in the data field
DOT     EQU 1

        ORG 2000H
START:  MVI A,00H
LOOP:   STA CURDT
        PUSH PSW
        MVI B,DOT
        CALL UPDDT      ; display A
        LXI D,8000H     ; short pause
WAIT:   DCX D
        MOV A,D
        ORA E
        JNZ WAIT
        POP PSW
        INR A
        JNZ LOOP
        RST 1           ; back to the monitor
        END START
