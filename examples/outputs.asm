; Sets all three 8155 ports to outputs and writes a fixed pattern.
; Open the I/O ports window, load this file, then press GO and EX:
;   PA = 55H (01010101), PB = F0H (11110000), PC = 2AH (101010)
; Press RS to set the ports back to inputs.

CSR     EQU 40H         ; 8155 command/status register
PA      EQU 41H
PB      EQU 42H
PC      EQU 43H
USCSR   EQU 40FFH       ; the monitor wants a copy of the CSR here

        ORG 2000H
START:  MVI A,0FH       ; PA, PB = outputs, PC = output (ALT2)
        OUT CSR
        STA USCSR
        MVI A,55H
        OUT PA
        MVI A,0F0H
        OUT PB
        MVI A,2AH
        OUT PC
        RST 1           ; back to the monitor
        END START
