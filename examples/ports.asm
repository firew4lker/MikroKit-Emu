; Copies the 8155 port B switches to the port A LEDs, forever.
; Open the I/O ports window, run with GO EX, and click the PB switches.
; Stop it with RS (reset).

CSR     EQU 40H         ; 8155 command/status register
PA      EQU 41H
PB      EQU 42H
USCSR   EQU 40FFH       ; the monitor wants a copy of the CSR here

        ORG 2000H
START:  MVI A,01H       ; PA = output, PB = input, PC = input
        OUT CSR
        STA USCSR
LOOP:   IN PB
        OUT PA
        JMP LOOP
        END START
