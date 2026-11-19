
;***************************************************************************
;
;	            MICROKIT MONITOR VER 5.12
;
;***************************************************************************

; ABSTRACT 2 SEPT 91
;
; THIS PROGRAM IS A SMALL MONITOR FOR THE MICROKIT AND PROVIDES A MINIMUM 
; LEVEL OF UTILITY FUNCTIONS FOR THE USER.
; THIS MONITOR PROGRAM WORKS EITHER WITH THE ON BOARD KEYBOARD/LED DISPLAY 
; OR INTER-ACTIVE WITH A TERMINAL.
; THE MONITOR ALLOWS THE USER TO PERFORM FUNCTIONS AS MEMORY AND REGISTER
; MANIPULATION, PROGRAM LOADING, PROGRAM EXECUTION, INTERRUPTION OF PROGRAM
; EXECUTION, SYSTEM RESET ETC.

; THE PROGRAM IS ORGANIZED AS FOLLOWS:
;       1)    EQUATE SYMBOLS
;       2)    COLD START ROUTINE (RESET)
;       3)    WARM START - REGISTER	SAVE ROUTINE
;       4)    INTERRUPT VECTORS
;       5)    KEYBOARD MONITOR
;       6)    CRT MONITOR
;       7)    TAPE WRITE ROUTINE FOR KEYBOARD MONITOR
;       8)    TAPE READ ROUTINE FOR KEYBOARD MONITOR
;       9)    DOWNLOAD ROUTINE FOR KEYBOARD MONITOR
;       10)   SELECT THE SERIAL PORT (FOR DOWNLOADING) FOR KEYBOARD MONITOR
;       11)   LAYOUT OF RAM USAGE
;***************************************************************************

; THE KEYBOARD OF MICROKIT HAS THE FOLLOWING APPEARANCE
;   ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐
;   │RESET │ │VECT  │ │ C    │ │ D    │ │ E    │ │ F    │	
;   │      │ │INTR  │ │      │ │      │ │      │ │      │
;   └──────┘ └──────┘ └──────┘ └──────┘ └──────┘ └──────┘
;   ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐
;   │SINGLE│ │  GO  │ │ 8    │ │ 9    │ │ A    │ │ B    │	
;   │ STEP │ │      │ │      │ │      │ │      │ │      │
;   └──────┘ └──────┘ └──────┘ └──────┘ └──────┘ └──────┘
;   ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐
;   │SUBST │ │ EXAM │ │ 4    │ │ 5    │ │ 6    │ │ 7    │	
;   │ MEM  │ │  REG │ │      │ │      │ │      │ │      │
;   └──────┘ └──────┘ └──────┘ └──────┘ └──────┘ └──────┘
;   ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐
;   │ NEXT │ │ EXEC │ │ 0    │ │ 1    │ │ 2    │ │ 3    │	
;   │  ,   │ │   .  │ │      │ │      │ │      │ │      │
;   └──────┘ └──────┘ └──────┘ └──────┘ └──────┘ └──────┘


;***************************************************************************
;
;	MONITOR EQUATE SYMBOLS - CONSTANT DECLARATION
;
;***************************************************************************

RAMST EQU 2000H ; Start address of 2 or 8 Kbytes of RAM.

; This program assumes that the RAM begins at address 2000H.
; The program assumes that a 256 Bytes extra RAM memory begins at 4000H
; and uses it for the program stack, temporary storage of monitor variables,
; and for saving user registers.

RMUSE   EQU 30	; RAM usage - Currently, 30 bytes are used for
                ; saving registers and variables.
                
SKLN EQU 06	; Reserved positions for the user programs.

UBRLN EQU 15    ; 5 user branches (3 bytes each) to the
                ; beginning of the interrupt routines.
ADFLD EQU 0     ; Indicates use of address field of display.

ADISP EQU 90H   ; Control character for the 8279 to inticate
                ; output to address field of display.                
DDISP EQU 94H   ; Control character for the 8279 to indicate output
                ; to data field of display.                
CNTRL EQU 2FH   ; Address for sending control characters to
                ; display IC (8279) - Control register of 8279. 
DSPLY EQU 2EH   ; Address for sending characters to display - Data
                ; register of 8279.              
COMMA EQU 11H   ; Comma from keyboard (this is the 1N1 key).

CSR_INIT EQU 00   ; Initial value for command/status register of 8155.
CSR_8155 EQU 40H  ; Output port for command/status register of 8155.
DOT EQU 1         ; Indicator for dot in display.
NODOT EQU 0       ; Indicator for no dot in display.
DTFLD EQU 1       ; Indicates use of data field of display.
DTMSK EQU 08H     ; Mask for turning on dot in display.
EMPTY EQU 80H     ; High order Bit 1 indicates empty input buffer.
KBNIT EQU 0CCH    ; Control character to set display output to
                  ; all ones during blanking period.
KMODE EQU 0       ; Control byte to set keyboard/display mode
                  ; (2 key rollover, 8 character left entry) for 8279. 
MKEY EQU 13H      ; Code for the 1M1 key (This code is used for moving
                  ; one position back in substitute memory command)
                  
MONSTK EQU RAMST+2000H+255-(RMUSE+UBRLN+SKLN)
                  ; Start of monitor stack.
                  
; NUMC - Defined later  ; Number of commands.
; NUMRG - Defined later ; Number of register save locations.

PERIOD EQU 10H    ; Period from keyboard (this is the 'EX' key).
PROMPT EQU 0FBH   ; Prompt character for display (Dash).
READF EQU 40H     ; Control byte to the 8279 for reading the keyboard.

TIMERHI EQU 45H   ; Output port for high order byte of timer value.
TIMERLO EQU 44H   ; Output port for low order byte of timer value.
TSTART EQU 0C0H   ; Two bits (11-000000) for starting the timer of 8155.
TSTOP EQU 40H     ; Two bits (01-000000) for stoping the timer of 8155.
TMR_COUNT EQU 197 ; Timer value for single step or trace commands.

SUBMC EQU 02      ; Code for substitute memory command.
UNMSK EQU 0EH     ; Unmask interrupts from the keyboard.

USERBR EQU RAMST+2000H+256-(RMUSE+UBRLN)
                  ; Start of user branch locations for interrupts.
                  
USERVR EQU MONSTK+1  ; Start of 6 locations for user variables.
UART_MODE EQU 8EH    ; Mode byte for Usart (8251).
UART_CMND EQU 27H    ; Command byte for the Usart (8251).
UART_CTL_ST EQU 1FH  ; Console control (output) / status (input)
                     ; port (Usart-8251).
UART_DATA EQU 1EH    ; Console input/output data port (Usart-8251).
MONSEL EQU 0FH       ; Monitor select input port.
MASK0 EQU 01H        ; Mask for bit 0.

XON EQU 11H          ; XON control character code.
XOFF EQU 13H         ; XOFF control character code.

;***************************************************************************
; EQUATE SYMBOLS FOR 'CRT MONITOR'.
;***************************************************************************

ESCAPE EQU 1BH     ; Code for Escape character.
CR EQU 0DH         ; Code for carriage return.
LF EQU 0AH         ; Code for line feed.
CTRL_C EQU 03H     ; Code for CTRL-C.
HCHAR EQU 0FH      ; Mask to select lower hex character from byte.
INVRT EQU 0FFH     ; Mask to invert half byte flag.
LOWER EQU 0        ; Denotes lower half of byte in ENTER_CMD.
; MNSTK EQU -      ; Start of monitor stack-defined in keyboard monitor.
; NCMDS EQU ---    ; Number of valid commands - defined later.
NEWLN EQU 0FH      ; Mask for checking memory address display.
PRTY0 EQU 07FH     ; Mask to clear parity bit from concole character.
; RAMST EQU -      ; Start address of RAM-defined in keyboard monitor.
; RTABS EQU -      ; Size of entry in RTAB table.
UPPER EQU 0FFH     ; Denotes upper half of byte in ENTER_CMD.
UART_RBR EQU 02H   ; Receiver buffer ready bit in USART.
UART_TRDY EQU 01H  ; Transmiter ready bit in USART.

;**************************************************************************
; EQUATE SYMBOLS FOR 'TAPE READ-WRITE ROUTINES'.
;**************************************************************************

CYCNO EQU 16       ; Twice the number of cycles per tone burst.
HAFCYC EQU 30      ; Determines tone frequency.
CKRATE EQU 14      ; Sets sample rate.
LEADER EQU 250     ; Basic number of succesive tone bursts at leader.
LDRCHK EQU 250     ; Used in playback to verify presence of leader.
TIMES EQU 5        ; Number of loops for creating the leader.
REPEAT EQU 50      ; Number of loops to verify leader.
DELY1 EQU 3        ; Loop count used with LDELAY for 2 sec delay.
DELY2 EQU 6        ; Loop count for 4 sec delay.

;***************************************************************************
; EQUATE SYMBOLS FOR THE 'DOWNLOAD ROUTINE'.
;***************************************************************************

GOOD_CODE EQU 00H  ; Code for good transmission.
ERR_CODE EQU 0FFH  ; Code for transmission error.

NBITS EQU 09H      ; Number of data bits forming an ASCII character.

;***************************************************************************
;***************************************************************************
;
; MICROKIT MONITOR PROGRAM
;
;***************************************************************************
;***************************************************************************

; ORG 0000 - "RESET" KEY ENTRY POINT (COLD START) - RST 0 ENTRY POINT

    MVI A,KMODE    ; Get mode control character for 8279.
    OUT CNTRL      ; Set keyboard/display mode.
    JMP CLDST      ; Go finish cold start.
    NOP

; **** RST 1 ENTRY POINT - WARM START ****

; All user's programs must end with a RST 1 instruction in order to
; return at this point. In that case the monitor will be able to save 
; user's register for evaluation.

    ORG 8
    
WRMST:              ; Enter warm start.
                    ; Save all user's CPU registers in memory.
    SHLD LSAV       ; Save HL registers so that you can use them.
    POP H           ; Get user program counter from top of stack.
    SHLD PSAV       ; Save it in memory.
    PUSH PSW        ; Put Flip/Flops and register A in stack.
    POP H           ; Get them in register HL.
    SHLD FSAV       ; Save Flip/Flops and register A in memory.
    LXI H,0000      ; Clear HL.
    DAD SP          ; Get user stack pointer in HL.
    SHLD SSAV       ; Save stack pointer.
    LXI H,BSAV+1    ; Use stack pointer for saving remaining registers.
    SPHL            ; Exchange HL with the SP.
    PUSH B          ; Save BC.
    PUSH D          ; Save DE.
    JMP WRMS1       ; Leave room for vectored interrupts.
    
    
;***************************************************************************

; THE RAM LOCATIONS 24H - 3EH ARE PREDEFINED TO HOLD THE JUMP VECTORS TO
; INTERRUPT ROUTINES.


;**** TRAP ENTRY POINT (FROM TIMER OF 8155) ****

    ORG 24H         ; Used for Single Step and Trace commands.
    JMP TRAP_SERV   ; Go back to Single Step or Trace routine.
    
; **** RST 5 ENTRY POINT ****

    ORG 28H
    JMP RSET5        ; Branch to RST 5 location in RAM.
    
; **** RST 5.5 ENTRY POINT (FROM KEYBOARD INPUT - 8279) ****

    ORG 2CH          ; Used by the keyboard controller (8279).
    JMP ININT        ; Branch to keyboard input interrupt routine.

; **** RST 6 ENTRY POINT ****

    ORG 30H
    JMP RSET6        ; Branch to RST 6 location in RAM.
    
; **** RST 6.5 ENTRY POINT (HARD WIRED TO J2 AND X1) ****

    ORG 34H          ; Not used by the monitor. Available to the user.
    JMP RST65        ; Branch to RST 6.5 location in RAM.
    
; **** RST 7 ENTRY POINT ****

    ORG 38H
    JMP RSET7        ; Branch to RST 7 location in RAM.
    
; **** RST 7.5 ENTRY POINT (HARD WIRED TO 1VI. KEY) ****

    ORG 3CH           ; Available to the user with 'VI* key.
    JMP USINT         ; Branch to user interrupt location in RAM.

;***************************************************************************

WRMS1:                ; Continue saving user status.
    RIM               ; Get user interrupt status and interrupt mask.
    ANI 8FH           ; Keep interrupt flag, interrupt mask and SID pin data.
    STA ISAV          ; Save interrupt status and mask in memory.

    MVI A,UNMSK      ; Unmask interrupts from the keyboard.
    SIM
    DI                ; Interrupts disabled while monitor is
                      ; running (except when waiting for input).
    IN MONSEL         ; Decide which monitor you will use.
                      ; Keyboard/Display monitor or CRT monitor ?
    ANI MASK0         ; Mask off all bits exept 0.
    JZ CRTMON         ; If bitO = 0 go to CRT monitor.
                      ; If it is 1 enter Keyboard monitor.
                      
;****************************************************************************
;
; KEYBOARD / DISPLAY MONITOR
;
;****************************************************************************

; THE KEYBOARD MONITOR BEGINS WITH THE COMMAND RECOGNIZER, FOLLOWED
; BY THE COMMAND EXECUTION ROUTINES SECTION, UTILITY ROUTINE SECTION
; AND MONITOR TABLES.
; THROUGHOUT THE KEYBOARD MONITOR, A COMMENT FIELD BEGINNING WITH
; "ARG" INDICATES A STATEMENT WHICH LOADS A VALUE INTO A
; REGISTER AS AN ARGUMENT FOR A FUNCTION.

; LIST OF KEYBOARD MONITOR ROUTINES 
;==================================
; CMMND   : COMMAND RECOGNIZER
;------
; EXAM       : EXAMINE AND MODIFY CPU REGISTERS
; GOCMD      : EXECUTE USER PROGRAM
; SSTEP      : EXECUTE ONLY ONE PROGRAM INSTRUCTION
; SUBST      : EXAMINE AND MODIFY RAM MEMORY LOCATIONS
;------
; CLEAR      : CLEAR DISPLAY
; CLDIS      : CLEAR DISPLAY AND TERMINATE COMMAND
; CLDST      : COLD START
; DISPC      : DISPLAY PROGRAM COUNTER
; ERR        : DISPLAY ERROR MESSAGE
; GTHEX      : GET HEX DIGITS FROM KEYBOARD
; HXDSP      : EXPAND HEX DIGITS FOR DISPLAY
; ININT      : RST5.5 INTERRUPT PROCESSING (KEYBOARD)
; INSDG      : INSERT HEX DIGIT
; NXTRG      : ADVANCE POINTER TO NEXT REGISTER
; OUTPT      : OUTPUT CHARACTERS TO DISPLAY
; RDKBD      : READ KEYBOARD
; RETF       : RETURN FALSE
; RETT       : RETURN TRUE
; RGLOC      : GET REGISTER SAVE LOCATION
; RGNAM      : DISPLAY REGISTER NAME
; RSTOR      : RESTORE USER REGISTERS - JUMP TO USER PROGRAM
; TRAP_SERV  : TRAP SERVICE ROUTINE - JUMPS TO PROPER MONITOR ROUTINE.
; UPDAD      : UPDATE ADDRESS FIELD OF DISPLAY
; UPDDT      : UPDATE DATA FIELD OF DISPLAY

;***************************************************************************

KBRMON:             ; Output SIGN-ON message.
    MVI A,ADFLD      ; ARG - Use address field of display.
    MVI B,NODOT      ; ARG - No dot in address field.
    LXI H,SGNAD      ; ARG - Get address of address field
                     ; part of sign-on message.
    CALL OUTPT       ; Output sign-on message to address field.
    MVI A,DTFLD      ; ARG - Use data field of display.
    MVI B,NODOT      ; ARG - No dot in data field.
    LXI H,SGNDT      ; ARG - Get address of data field part
                     ; of sign-on message.
    CALL OUTPT       ; Output sign-on message to data field.
    MVI A,EMPTY
    STA IBUFF       ; Set input buffer empty flag.

;****************************************************************************

; FUNCTION: CMMND - COMMAND RECOGNIZER
; INPUTS  : NONE 
; OUTPUTS : NONE
; CALLS   : RDKBD,ERR,SUBST,EXAM,GOCMD.SSTEP 
; DESTROYS: A,B,C,D,E,H,L,F/F

CMMND:
    LXI H,MNSTK-2    ; Allways initialize monitor stack pointer 
                     ; to the first location.
    SPHL             ; Output prompt character to display.
    MVI A,ADISP     ; Get address for control character.
    OUT CNTRL        ; Output control character to use address field.
    MVI A,PROMPT     ; Address for output character.
    OUT DSPLY        ; Output prompt character.
    CALL RDKBD       ; Read keyboard.
    LXI B,NUMC       ; Counter for number of commands in reg. C.
    LXI H,CMDTB      ; Get address of command table.
CMD10:
    CMP M            ; Recognize the command ?
    JZ CMD15         ; Yes - Go and process it.
    INX H            ; No - Next command table entry.
    DCR C            ; End of command table ?
    JNZ CMD10        ; No - Go check next entry.
                     ; Yes - Command unknown.
    JMP ERR          ; Display error message - Get another command.
CMD15:
    LXI H,CMDAD      ; Get address of command address table.
    DCR C            ; Adjust command counter. Counter acts as pointer
                     ; to command address table.
    DAD B            ; Add pointer to command address table.
    DAD B            ; Table has 2 byte entries.
    MOV A,M          ; Get low order byte of command address.
    INX H
    MOV H,M          ; Get high order byte of command address in H.
    MOV L,A          ; Put low order byte in L.
                     ; Command routine address is now in H,L.
    PCHL             ; Branch to address in H,L. This is the
                     ; begining of the selected command.
                     
;***************************************************************************
;
; KEYBOARD/DISPLAY MONITOR COMMAND ROUTINES
;
;**********************************************************************"****

; FUNCTION: EXAM - EXAMINE AND MODIFY REGISTERS 
; INPUTS  : NONE 
; OUTPUTS : NONE
; CALLS   : CLEAR,SETRG,ERR,RGNAM,RGLOC,UPDDT,GTHEX,NXTRG 
; DESTROYS: A,B,C,D,E,H,L,F/F

EXAM:
    MVI B,NODOT      ; ARG - No dot in address field of display.
    CALL CLEAR       ; Clear display.
    MVI A,00         ; Set value to point at the register A.
    STA RGPTR        ; Save value in register pointer.
EXM05:
    CALL RGNAM       ; Output register name to address field.
    CALL RGLOC       ; Get register save location in H,L.
    MOV A,M          ; Get register contents.
    STA CURDT        ; Store register contents at current data.
    MVI B,DOT        ; ARG - Dot in data field.
    CALL UPDDT       ; Update data field of display.
    MVI B,DTFLD      ; ARG - Use data field of display.
    CALL GTHEX       ; Get hex digits. Were any digits received ?
    JNC EXM10        ; No - Do not update register contents.
    CALL RGLOC       ; Yes - Get register save location in H,L.
    MOV M,E          ; Update register contents.
EXM10:
    CPI PERIOD       ; Was last character a period ?
    JZ CLDIS         ; Yes - Clear display and terminate command.
    CPI COMMA        ; Was last character comma 1?
    JNZ ERR          ; No - Display error msg. and terminate command.
    CALL NXTRG       ; Yes - Advance register pointer to next register.
    JMP EXM05        ; Continue processing with next register.
    
;***************************************************************************

; FUNCTION: GOCMD - EXECUTE USER PROGRAM 
; INPUTS  : NONE 
; OUTPUTS : NONE
; CALLS   : DISPC,RDKBD,CLEAR,GTHEX,ERR,OUTPT 
; DESTROYS: A,B,C,D,E,H,L,F/F

GOCMD:
    CALL DISPC       ; Display user program counter.
    CALL RDKBD       ; Read from keyboard.
    CPI PERIOD       ; Is character a period ?
    JZ G10           ; Yes - Go execute the command.
                     ; No - ARG - Character is still in A.
    STA IBUFF       ; Replace character in input buffer.
    MVI B,DOT        ; ARG - Dot in address field.
    CALL CLEAR       ; Clear display.
    MVI B,ADFLD     ; ARG - Use address field.
    CALL GTHEX       ; Get hex digits.
    CPI PERIOD       ; Was last character a period ?
    JNZ ERR          ; No - Display error message and terminate command.
    XCHG             ; Put hex value from GTHEX to H,L.
    SHLD PSAV        ; Hex value is new user PC.
G10:
    MVI B,NODOT      ; Yes - ARG - No dot in address field.
    CALL CLEAR       ; Clear display.
    MVI A,ADFLD      ; ARG - Use address field of display.
    MVI B,NODOT      ; ARG - No dot in address field.
    LXI H,EXMSG      ; Get address of execution message in H,L.
    CALL OUTPT       ; Display execution message .
    JMP RSTOR        ; Restore user registers including Program Counter.
                     ; Begin execution of user program.

;***************************************************************************

; FUNCTION: SSTEP - SINGLE STEP (EXECUTE ONE USER INSTRUCTION)
; INPUTS  : NONE 
; OUTOUTS : NONE
; CALLS   : DISPC,RDKBD,CLEAR,GTHEX,ERR 
; DESTROYS: A,B,C,D,E,H,L,F/F

SSTEP:
    CALL DISPC       ; Display user program counter.
    CALL RDKBD       ; Read from keyboard.
    CPI PERIOD       ; Was character a period ?
    JZ CLDIS         ; Yes - Clear display and terminate command.
    CPI COMMA        ; Was last character a comma 1,1 ?
    JZ STP20         ; Yes - Go set timer.
                     ; No - Character from keyboard was neither ; period nor comma.
    STA IBUFF        ; Replace the character in the input buffer.
    MVI B,DOT        ; ARG - Dot in address field.
    CALL CLEAR       ; Clear display.
    MVI B,ADFLD      ; ARG - Use address field of display.
    CALL GTHEX       ; Get hex digits - were any digits received ?
    JNC ERR          ; No - Display error msg. and terminate command.
    XCHG             ; Hex value from GTHEX to HL.
    SHLD PSAV        ; Hex value is new user PC.
    CPI PERIOD       ; Was last character from GTHEX a period ?
    JZ CLDIS         ; Yes - Clear display and terminate command.
                     ; No - Must have been a comma.

STP20:
    LDA ISAV         ; Get user interrupt mask.
    ANI 08H          ; Keep interrupt status.
    STA TEMP         ; Save user interrupt status.
    LHLD PSAV        ; Get user PC.
    MOV A,M          ; Get user instruction.
    CPI 0F3H         ; DI instruction ?
    JNZ STP21        ; No.
    XRA A            ; Yes - Reset user interrupt status.
    JMP STP22
STP21:
    CPI 0FBH         ; El instruction ?
    JNZ STP23        ; No.
    MVI A,08H        ; Yes - Set user interrupt status.
STP22:
    STA TEMP         ; Save new user interrupt status.
STP23:               ; Start timer - Jump to user program.
    MVI A,80H        ; High order bits of timer value logicaly
    OUT TIMERHI      ; OR'ed with timer mode.
    MVI A,TMR_COUNT  ; Low order bits of timer value (197 CPU states). 
    OUT TIMERLO
    LDA USCSR        ; Get user image of what is in CSR.
    ORI TSTART       ; Set timer command bits to start timer.
    OUT CSR_8155     ; Start timer.
    JMP RSTOR        ; Restore user registers - Start executing user's
                     ; program - When the first instruction wil be
                     ; executed an interrupt will occur if you are in
                     ; Single Step or Trace commands.


STP25:               ; TRAP service routine for CRT monitor.
                     ; Branch here when 8155 timer interrupts after ; one user instruction.
  
    LDA USCSR        ; Get user image of what's in CSR.
    ANI 3FH          ; Clear 2 high order bits.
    ORI TSTOP        ; Set timer stop bits.
    OUT CSR_8155     ; Stop timer.
    POP PSW          ; Retrieve PSW.
    SHLD LSAV        ; Save HL.
    POP H            ; Get user program counter from top of stack.
    SHLD PSAV        ; Save user PC.
    PUSH PSW
    POP H
    SHLD FSAV        ; Save Flip/Flops and A register.
    LXI H,0          ; Clear HL.
    DAD SP           ; Get user stack pointer.
    SHLD SSAV        ; Save user stack pointer.
    LXI H,BSAV+1     ; Set monitor stack pointer for saving remaining
                     ; user registers.
    SPHL
    PUSH B           ; Save BC.
    PUSH D           ; Save DE.
    RIM              ; Get user interrupt mask.
    ANI 87H          ; Keep interrupt mask bits and SID pin data.
    LXI H,TEMP       ; Get user interrupt status.
    ORA M            ; OR it into mask.
    STA ISAV         ; Save interrupt status and mask.
    MVI A,UNMSK      ; Unmask interrupts from the keyboard.
    SIM
    JMP SSTEP        ; Go get ready for another instruction. 

;***************************************************************************

; FUNCTION: SUBST - SUBSTITUTE MEMORY 
; INPUTS  : NONE 
; OUTPUTS : NONE
; CALLS   : CLEAR,GTHEX,UPDAD,UPDDT,ERR 
; DESTROYS: A,B,C,D,E,H,L,F/F

SUBST:
    MVI B,DOT        ; ARG - Dot in adderss field.
    CALL CLEAR       ; Clear the display.
    MVI B,ADFLD      ; ARG - Use address field of display.
    CALL GTHEX       ; Get hex digits. Were any digits received ?
    JNC ERR          ; No - Display error message and terminate command.
    XCHG             ; Assign hex value returned by GTHEX
    SHLD CURAD       ; to current address.
    PUSH PSW         ; Save register A.
    MVI A,SUBMC      ; Put command code in A reg.
    STA SVCMC        ; Save it in memory. You are in the substitute
                     ; memory command.
    POP PSW          ; Restore register A.
SUB05:
    CPI COMMA        ; Was the last character from keyboard ?
    JZ SUB07         ; Yes - Continue the command.
    CPI MKEY         ; No - Was 1M' the last character from keyboard ?
    JNZ SUB15        ; No - Go and terminate the command.
SUB07:
                     ; Yes - The last character was 1or 1M..
    MVI B,NODOT      ; ARG - No dot in address field.
    CALL UPDAD       ; Update address field of display.
    LHLD CURAD       ; Get current address in H,L.
    MOV A,M          ; Get data byte pointed by the current address.
    STA CURDT        ; Store data byte in current data storage location.
    MVI B,DOT        ; ARG - Dot in data field.
    CALL UPDDT       ; Update data field of display.
    MVI B,DTFLD      ; ARG - Use data field.
    CALL GTHEX       ; Get hex digits - Were any hex digits received ?
    STA AIMAG        ; Save last character in memory.
    JNC SUB10        ; No - Leave data unchanged.
    LHLD CURAD       ; Yes - Get current address in H,L.
    MOV M,E          ; Store new data at current address.
                     ; Make sure data was actualy stored in case 
                     ; current address is in EPROM or is not existing. 
    MOV A,E          ; Data to register A for comparison.
    CMP M            ; Was data stored corectly ?
    JNZ SUB20        ; No - Terminate command with error message.
    LDA AIMAG        ; Restore register A.
SUB10:
    LHLD CURAD       ; Increment current address.
    INX H
    CPI MKEY         ; Was last character 'M' key.
    JNZ SUB12        ; No - So increment address.
    DCX H            ; Yes - Decrement address by 1.
    DCX H            ; Two times to cancel first INX H.
SUB12:
    SHLD CURAD
    JMP SUB05        ; Continue the command with new address.
SUB15:
    CPI PERIOD       ; Was last character 1.' ?
    JNZ SUB20        ; No - Terminate command with error messagge.
    MVI A,0          ; Yes - Clear command code memory location.
    STA SVCMC
    JMP CLDIS        ; Clear display and terminate command.
SUB20:
    MVI A,0          ; Clear command code memory location.
    STA SVCMC
    JMP ERR          ; Display error message and terminate command.
    
;***************************************************************************
;
; KEYBOARD/DI SPLAY MONITOR UTILITY ROUTINES
;
;***************************************************************************

; FUNCTION   : CLEAR - CLEAR THE DISPLAY
; INPUTS     : B-DOT FLAG: 1 MEANS PUT DOT IN ADDRESS FIELD OF DISPLAY I 
;                        : 0 MEANS NO DOT
; OUTPUTS    : NONE 
; CALLS      : OUTPT 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: CLEAR SENDS BLANK CHARACTERS TO BOTH THE ADDRESS ;
;              FIELD AND THE DATA FIELD OF THE DISPLAY. IF THE
;              DOT FLAG IS SET THEN a DOT UILL APPEAR AT THE
;              RIGHT EDGE OF THE ADDRESS FIELD.

CLEAR:
    MVI A,ADFLD      ; ARG - Use address field of display.
                     ; ARG - Flag for dot in address field is in B.
    LXI H,BLNKS      ; ARG - Address of blanks for display.
    CALL OUTPT       ; Output blanks to address field.
    MVI A,DTFLD      ; ARG - Use data field of display.
    MVI B,NODOT      ; ARG - No dot in data field.
    LXI H,BLNKS      ; ARG - Address of blanks for display.
    CALL OUTPT       ; Output blanks to data field.
    RET              ; Return.

;***************************************************************************

; FUNCTION   : CLDIS - CLEAR DISPLAY AND TERMINATE COMMAND
; INPUTS     : NONE
; OUTPUTS    : NONE
; CALLS      : CLEAR
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: COMMAND ROUTINES JUMP TO CLDIS IN ORDER TO 
;              TERMINATE NORMALLY. CLDIS CLEARS THE DISPLAY AND
;              BRANCHES TO THE COMMAND RECOGNISER.

CLDIS:
    MVI B,NODOT      ; ARG - No dot in address field.
    CALL CLEAR       ; Clear the display.
    JMP CMMND        ; Go get another command.
    
;***************************************************************************

; FUNCTION   : CLDST - COLD START
; INPUTS     : NONE
; OUTPUTS    : NONE
; CALLS      : NOTHING
; DESTROYS   : A
; DESCRIPTION: CLDST COMPLETES COLD START INITIALISATION, AND JUMPS BACK 
;              TO THE MAIN COLD START PROCEDURE.

CLDST:
    MVI A,KBNIT      ; Get control character.
    OUT CNTRL        ; Initialize keyboard/display blanking.
    MVI A,CSR_INIT   ; Initial value of command status register of 8155.
    OUT CSR_8155     ; Initialize CSR of 8155.
    STA USCSR        ; Initialize user CSR value at memory.
    MVI A,UART_MODE  ; Usart initialize byte for MODE.
    OUT UART_CTL_ST  ; Output to USART control port (8251).
    MVI A,UART_CMND  ; USART initialize COMMAND byte.
    OUT UART_CTL_ST  ; Output to console control port (8251).
    MVI A,00         ; Clear memory location for command code.
    STA SVCMC
    
    MVI A,02         ; Set default active download port.
    STA DPORT
    
    LXI SP,MNSTK-2   ; Initialize monitor stack pointer.
    JMP WRMST        ; Back to main procedure.

;***************************************************************************

; FUNCTION   : DISPC - DISPLAY PROGRAM COUNTER 
; INPUTS     : NONE 
; OUTPUTS    : NONE 
; CALLS      : UPDAD,UPDDT 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: DISPC DISPLAYS THE USER PROGRAM COUNTER IN THE 
;              ADDRESS FIELD OF THE DISPLAY, WITH A DOT AT THE
;              RIGHT EDGE OF THE FIELD. THE BYTE OF DATA ADDRESSED
;              BY THE PROGRAM COUNTER IS DISPLAYED IN THE DATA
;              FIELD OF THE DISPLAY.

DISPC:
    LHLD PSAV        ; Get user program counter.
    SHLD CURAD       ; Make it the current address.
    MOV A,M          ; Get the instruction at that address.
    STA CURDT        ; Make it the currend data.
    MVI B,DOT        ; ARG - Dot in address field.
    CALL UPDAD       ; Update address field of display.
    MVI B,NODOT      ; ARG - No dot in data field.
    CALL UPDDT       ; Update data field of display.
    RET

;***************************************************************************

; FUNCTION   : ERR - DISPLAY ERROR MESSAGE
; INPUTS     : NONE
; OUTPUTS    : NONE
; CALLS      : OUTPT
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: ERR IS JUMPED TO BY COMMAND ROUTINES WISHING TO
;              TERMINATE BECAUSE OF AN ERROR.
;              ERR OUTPUTS AN ERROR MESSAGE TO THE DISPLAY AND
;              BRANCHES TO THE COMMAND RECOGNISER.

ERR:
    MVI A,ADFLD ; ARG - Use address field.
    MVI B,NODOT ; ARG - No dot in address field.
    LXI H,ERMSG ; ARG - Address of error message.
    CALL OUTPT ; Output error message to address field.
    MVI A,DTFLD ; ARG - Use data field.
    MVI B,NODOT ; ARG - No dot in data field.
    LXI H,BLNKS ; ARG - Address of blanks for display.
    CALL OUTPT ; Output blanks to data field.
    JMP CMMND ; Go get a new command.
    
;*************************************************************************** 

; FUNCTION   : GTHEX - GET HEX DIGITS
; INPUTS     : B - DISPLAY FLAG - 0 MEANS USE ADDRESS FIELD OF DISPLAY
;                               - 1 MEANS USE DATA FIELD OF DISPLAY
; OUTPUTS    : A - LAST CHARACTER READ FROM KEYBOARD
;              DE - HEX DIGITS FROM KEYBOARD EVALUATED MODULO 2**16
;              CARRY - SET IF AT LEAST ONE VALID HEX DIGIT WAS READ
;                 - RESET OTHERWISE
; CALLS      : RDKBD,INSDG.HXDSP,OUTPT 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: GTHEX ACCEPTS A STRING OF HEX DIGITS FROM THE KEYBOARD,
;              DISPLAYS THEM AS THEY ARE RECEIVED, AND RETURNS THEIR
;              VALUE AS A 16 BIT INTEGER. IF MORE THAN 4 HEX DIGITS
;              ARE RECEIVED, ONLY THE LAST 4 ARE USED. IF THE DISPLAY
;              FLAG IS SET, THE LAST 2 HEX DIGITS ARE DISPLAYED IN
;              THE DATA FIELD OF THE DISPLAY. OTHERWISE, THE LAST 4 
;              HEX DIGITS ARE DISPLAYED IN THE ADDRESS FIELD OF THE
;              DISPLAY. IN EITHER CASE, A DOT WILL BE DISPLAYED AT
;              THE RIGHTMOST EDGE OF THE FIELD, Aa CHARACTER WHICH IS
;              NOT A HEX DIGIT TERMINATES THE STRING AND IS RETURNED
;              AS AN OUTPUT OF THE FUNCTION. IF THE TERMINATOR IS NOT
;              A PERIOO OR A COMMA THEN ANY HEX DIGITS WHICH MAY HAVE
;              BEEN RECEIVED ARE CONSIDERED TO BE INVALID. THE ROUTINE
;              SETS THE CARRY FLAG TO INDICATE WHETHER OR NOT ANY VALID
;              HEX DIGITS WERE RECEIVED. THIS ROUTINE ACCEPTS THE KEY
;              'M' AS A VALID TERMINATOR ONLY WHEN WE ARE IN THE SUBSTITUTE
;              MEMORY COMMAND.

GTHEX:
    MVI C,0          ; Reset hex digit flag.
    PUSH B           ; Save display and hex digit flags.
    LXI D,0          ; Set hex value to zero.
    PUSH D           ; Save hex value.
GTH05:
    CALL RDKBD       ; Read keyboard.
    CPI 10H          ; Is character a hex digit ? (Digit values are 00-0F)
    JNC GTH20        ; No - Go check for terminator.
                     ; Yes - ARG- New hex digit is in A.
    POP D            ; ARG - Retrieve hex value.
    CALL INSDG       ; Insert new digit in hex value.
    POP B            ; Retrieve display flag.
    MVI C,1          ; Set hex digit flag (a hex digit has been read).
    PUSH B           ; Save display and hex digit flags.
    PUSH D           ; Save hex value.
    MOV A,B          ; Test display flag.
    RRC              ; Should address field of display be used ?
    JNC GTH10        ; Yes - Use hex value as it is.
                     ; No - Only low byte of hex value should
                     ; be used for data field of display.
    MOV D,E          ; Put low order byte of hex value in D.
GTH10:
                     ; ARG - Hex value to be expanded is in D,E.
    CALL HXDSP       ; Expand hex value for display.
                     ; ARG - Address of expanded hex value in H,L.
    MOV A,B          ; ARG - Put display flag in A.
    MVI B,DOT        ; ARG - Dot in appropriate field.
    CALL OUTPT       ; Output hex value to display.
    JMP GTH05        ; Go get next character.
GTH20:               ; Last character was not a hex digit.
    POP D            ; Retrieve hex value.
    POP B            ; Retrieve hex digit flag in C.
    CPI COMMA        ; Was last character ?
    JZ GTH25         ; Yes - Ready to return.
    CPI PERIOD       ; No - Was last character ?
    JZ GTH25         ; Yes - Ready to return.
    CPI MKEY         ; No - Was last character "M" key ?
    JZ GTH30         ; Yes - Examine in which command you are.
                     ; No - Invalid terminator-ignore any hex digits read.
GTH22:
    LXI D,0          ; Set hex value to zero.
    JMP RETF         ; Return false.
GTH25:
    MOV B,A          ; Save last character.
    MOV A,C          ; Shift hex digit flag to carry bit.
    RRC
    MOV A,B          ; Restore last character.
    RET              ; Return.
GTH30:
    LDA SVCMC        ; In this location there is the command code.
    CPI SUBMC        ; Are you in substitute memory command ?
    JNZ GTH22        ; No - So "M" is an invalid separator.
    MVI A,MKEY       ; Yes - Restore last character from keyboard.
    JMP GTH25        ; Restore carry flag and finish.
    
;***************************************************************************

; FUNCTION   : HXDSP - EXPAND HEX DIGITS FOR DISPLAY 
; INPUTS     : DE - 4 HEX DIGITS 
; OUTPUTS    : HL - ADDRESS OF OUTPUT BUFFER 
; CALLS      : NOTHING 
; DESTROYS   : A,H,L,F/F
; DESCRIPTION: HXDSP EXPANDS EACH INPUT BYTE TO 2 BYTES IN A 
;              FORM SUITABLE FOR DISPLAY BY THE OUTPUT ROUTINES.
;              EACH INPUT BYTE IS DIVIDED INTO 2 HEX DIGITS. EACH
;              HEX DIGIT IS PLACED IN THE LOU ORDER 4 BITS OF A
;              BYTE WHOSE HIGH ORDER 4 BITS ARE SET TO ZERO. THE
;              RESULTING BYTE IS STORED IN THE OUTPUT BUFFER.
;              THE FUNCTION RETURNS THE ADDRESS OF THE OUTPUT
;              BUFFER.

HXDSP:
    MOV A,D          ; Get first data byte.
    RRC              ; Convert 4 high order bits
    RRC              ; to a single character.
    RRC
    RRC
    ANI 0FH
    LXI H,OBUFF      ; Get address of output buffer.
    MOV M,A          ; Store character in output buffer.
    MOV A,D          ; Get first data byte and convert 4 low
    ANI 0FH          ; order bits to a single character.
    INX H            ; Next buffer position.
    MOV M,A          ; Store character in buffer.
    MOV A,E          ; Get second data byte.
    RRC              ; Convert 4 high order bits to a single character.
    RRC
    RRC
    RRC
    ANI 0FH
    INX H            ; Next buffer position.
    MOV M,A          ; Store character in buffer.
    MOV A,E          ; Get second data byte and convert low
    ANI 0FH          ; order 4 bits to a single character.
    INX H            ; Next buffer position.
    MOV M,A          ; Store character in buffer.
    LXI H,OBUFF      ; Return address of output buffer in H,L.
    RET
    
;***************************************************************************

; FUNCTION: ININT - KEYBOARD INTERRUPT PROCESSING 
; INPUTS     : NONE 
; OUTPUTS    : NONE 
; CALLS      : NOTHING 
; DESTROYS   : NOTHING
; DESCRIPTION: ININT IS ENTERED BY MEANS OF AN INTERRUPT VECTOR 
;              (RST 5.5) WHEN THE READ KEYBOARD ROUTINE IS WAITING
;              FOR A CHARACTER AND THE USER HAS PRESSED A KEY ON
;              THE KEYBOARD (EXCEPT "RESET" OR "VECTOR INTERRUPT").
;              ININT STORES THE INPUT CHARACTER IN THE INPUT BUFFER
;              AND RETURNS CONTROL TO THE READ KEYBOARD ROUTINE.

ININT:
    PUSH H           ; Save H, L.
    PUSH PSW         ; Save F/F and register A.
    MVI A,READF      ; Control character for reading the keyboard.
    OUT CNTRL        ; Send control character to control register of 8279.
    IN DSPLY         ; Get code of the pressed key.
    ANI 3FH          ; Zero 2 high order bits.
    STA IBUFF        ; Store character in input buffer.
    POP PSW          ; Restore F/F and register A.
    POP H            ; Restore H,L.
    RET

;***************************************************************************

; FUNCTION   : INSDG - INSERT HEX DIGIT 
; INPUTS     : A - HEX DIGIT TO BE INSERTED 
;              DE - HEX VALUE
; OUTPUTS    : DE - HEX VALUE WITH DIGIT INSERTED 
; CALLS      : NOTHING 
; DESTROYS   : A, F/F
; DESCRIPTION: INSDG SHIFTS THE CONTENTS OF REGISTERS D,E LEFT 4 BITS 
;              (1 HEX DIGIT) AND INSERTS THE HEX DIGIT IN REGISTER A
;              IN THE LOW ORDER DIGIT POSITION OF THE RESULT.
;              REGISTER A IS ASSUMED TO CONTAIN A SINGLE HEX DIGIT
;              IN THE LOW ORDER 4 BITS AND ZEROS IN THE HIGH ORDER
;              4 BITS.

INSDG:
    XCHG             ; Put D,E in H,L.
    DAD H            ; Shift H,L left 4 bits.
    DAD H
    DAD H
    DAD H
    ADD L            ; Insert low order digit.
    MOV L,A
    XCHG             ; Put H,L back in D,E.
    RET
    
;***************************************************************************

; FUNCTION: NXTRG - ADVANCE REGISTER POINTER TO NEXT REGISTER 
; INPUTS     : NONE
; OUTPUTS    : CARRY - 1 IF POINTER IS ADVANCED SUCCESSFULLY 
;                    - 0 OTHERWISE
; CALLS      : NOTHING 
; DESTROYS   : A,F/F
; DESCRIPTION: IF THE REGISTER POINTER POINTS TO THE LAST REGISTER 
;              IN THE EXAMINE REGISTER SEQUENCE, THE POINTER IS NOT
;              CHANGED AND THE FUNCTION RETURNS FALSE.
;              IF THE REGISTER POINTER DOES NOT POINT TO THE LAST
;              REGISTER THEN THE POINTER IS ADVANCED TO THE NEXT
;              REGISTER IN THE SEQUENCE AND THE FUNCTION RETURNS
;              TRUE.

NXTRG:
    LDA RGPTR        ; Get register pointer.
    CPI NUMRG-1      ; Does pointer point to last register ?
    JNC NXT05        ; Yes - Set pointer to the beginning.
    INR A            ; No - Advance register pointer.
    STA RGPTR        ; Save register pointer.
    RET
NXT05:
    MVI A,00         ; Set register pointer to register A.
    STA RGPTR        ; Save value.
    RET

;***************************************************************************

; FUNCTION: OUTPT - OUTPUT CHARACTERS TO DISPLAY 
; INPUTS  : A - DISPLAY FLAG -0 = USE ADDRESS FIELD OF DISPLAY 
;                            -1 = USE DATA FIELD OF DISPLAY
;           B - DOT FLAG -1 = OUTPUT DOT AT RIGHT EDGE OF FIELD
;                        -0 = NO DOT
;           HL - ADDRESS OF CHARACTERS TO BE DISPLAYED
; CALLS   : NOTHING 
; DESTROYS: A,B,C,D,E,H,L,F/F
; DESCRIPTION: OUTPUT SENDS CHARACTERS TO THE DISPLAY. THE ADDRESS 
;              OF THE CHARACTERS IS RECEIVED AS AN ARGUMENT IN H,L.
;              THE CHARACTERS MUST BE IN HEX FORMAT. EITHER 2
;              CHARACTERS ARE SEND TO THE DATA FIELD, OR 4
;              CHARACTERS ARE SEND TO THE ADDRESS FIELD, DEPENDING
;              ON THE DISPLAY FLAG ARGUMENT. THE DOT FLAG ARGUMENT
;              DETERMINES WHETHER OR NOT A DOT (DECIMAL POINT)
;              WILL BE SEND ALONG WITH THE LAST OUTPUT CHARACTER.

OUTPT:
    RRC              ; Use data field ?
    JC OUT05         ; Yes - Go set up to use data field.
    MVI C,4          ; No - Count for address field.
    MVI A,ADISP      ; Control character for output to address
                     ; field of display.
    JMP OUT10
OUT05:
    MVI C,2          ; Count for data field.
    MVI A,DDISP      ; Control character for output to data
                     ; field of display.
OUT10:  
    OUT CNTRL
OUT15:
    MOV A,M          ; Get output character.
    XCHG             ; Save output character address in D,E.
    LXI H,DSPTB      ; Get display format table address.
    ADD L            ; Use output character as a pointer to
    MOV L,A          ; display format table.
    MOV A,M          ; Get display format character from table.
    MOV H,C          ; Test counter without changing it.
    DCR H            ; Is this the last character ?
    JNZ OUT20        ; No - Go output character as it is.
    DCR B            ; Yes - Is dot flag set ?
    JNZ OUT20        ; No - Go output character as it is.
    ORI DTMSK        ; Yes - OR with DOT MASK in order to display
                     ; a dot with last character.
OUT20:
    CMA              ; Complement output character.
    OUT DSPLY        ; Send character to display.
    XCHG             ; Retrieve output character address.
    INX H            ; Next output character.
    DCR C            ; Any more characters ?
    JNZ OUT15        ; Yes - Go process another character.
    RET              ; No - Return.

;***************************************************************************
;
; FUNCTION: RDKBD - READ KEYBOARD 
; INPUTS     : NONE
; OUTPUTS    : A - CHARACTER READ FROM KEYBOARD 
; CALLS      : NOTHING 
; DESTROYS   : A,H,L,F/F
; DESCRIPTION: RDKBD DETERMINES WHETHER OR NOT THERE IS A CHARACTER 
;              IN THE INPUT BUFFER. IF NOT, THE FUNCTION ENABLES
;              INTERRUPTS AND LOOPS UNTIL THE INPUT INTERRUPT
;              ROUTINE STORES A CHARACTER IN THE BUFFER.
;              WHEN THE BUFFER CONTAINS A CHARACTER, THE FUNCTION
;              FLAGS THE BUFFER AS EMPTY AND RETURNS THE
;              CHARACTER AS OUTPUT TO ACCUMULATOR.

RDKBD:
    LXI H,IBUFF      ; Get input buffer address.
    MOV A,M          ; Get buffer contents.
                     ; High order bit = 1 means buffer is empty.
    ORA A            ; Is a character available ?
    JP RDK10         ; Yes - Exit from loop.
    EI               ; No - Ready for character from keyboard.
    JMP RDKBD
RDK10:
    MVI M,EMPTY      ; Set buffer empty flag.
    DI               ; Return with interrupts disabled.
    RET

;***************************************************************************

; FUNCTION   : RETF - RETURN FALSE 
; INPUTS     : NONE 
; OUTPUTS    : CARRY = 0 (FALSE)
; CALLS      : NOTHING
; DESTROYS   : CARRY
; DESCRIPTION: RETF IS JUMPED TO BY ROUTINES WISHING TO RETURN FALSE.
;              RETF RESETS CARRY TO 0 AND RETURNS TO THE CALLING ROUTINE.

RETF:
    STC              ; Set carry true.
    CMC              ; Complement carry to make it false.
    RET

;***************************************************************************

; FUNCTION   : RETT - RETURN TRUE
; INPUTS     : NONE
; OUTPUTS    : CARRY = 1 (TRUE)
; CALLS      : NOTHING
; DESTROYS   : CARRY
; DESCRIPTION: RETT IS JUMPED TO BY ROUTINES WISHING TO RETURN TRUE.
;              RETT SETS CARRY TO 1 AND RETURNS TO THE CALLING ROUTINE.

RETT:
    STC ; Set carry true.
    RET

;***************************************************************************

; FUNCTION   : RGLOC - GET REGISTER SAVE LOCATION 
; INPUTS     : NONE
; OUTPUTS    : HL - REGISTER SAVE LOCATION 
; CALLS      : NOTHING 
; DESTROYS   : B,C,H,L,F/F
; DESCRIPTION: RGLOC RETURNS THE SAVE LOCATION OF THE REGISTER 
;              INDICATED BY THE CURRENT REGISTER POINTER VALUE.

RGLOC:
    LHLD RGPTR       ; Get register pointer byte in H,L.
    MVI H,0          ; Register pointer is one byte long. So H=0.
    LXI B,RGTBL      ; Get register save location table address.
    DAD B            ; Pointer indexes table.
    MOV L,M          ; Get low order byte of register save location.
    MVI H,(MNSTK >> 8) ; Get high order byte of register save location.
    RET

;***************************************************************************

; FUNCTION: RGNAM - DISPLAY REGISTER NAME
; INPUTS     : NONE
; OUTPUTS    : NONE
; CALLS      : OUTPT
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: RGNAM DISPLAYS, IN THE ADDRESS FIELD OF THE DISPLAY,
;              THE REGISTER NAME CORRESPONDING TO THE CURRENT
;              REGISTER POINTER VALUE.

RGNAM:
    LHLD RGPTR       ; Get register pointer.
    MVI H,0
    DAD H            ; Multiply pointer value by 4.
    DAD H            ; (Register name table has 4 byte entries).
    LXI B,NMTBL      ; Get address of start of register name table.
    DAD B            ; ARG - Add table address to pointer - Result
                     ; is address of appropriate register name in H, L.
    MVI A,ADFLD      ; ARG - Use address field of display.
    MVI B,NODOT      ; ARG - No dot in address field.
    CALL OUTPT       ; Output register name to address field.
    RET

;***************************************************************************

; FUNCTION   : RSTOR - RESTORE USER REGISTERS 
; INPUTS     : NONE 
; OUTPUTS    : NONE 
; CALLS      : NOTHING 
; DESTROYS   : A,B,C,D,E,H,L,F,/F'S
; DESCRIPTION: RSTOR RESTORES ALL CPU REGISTERS, FLIP/FLOPS,
;              INTERRUPT STATUS, INTERRUPT MASK, STACK POINTER
;              AND PROGRAM COUNTER FROM THEIR RESPECTIVE SAVE
;              LOCATIONS IN MEMORY.
;              BY RESTORING THE PROGRAM COUNTER, THE ROUTINE
;              EFFECTIVELY TRANSFERS CONTROL TO THE ADDRESS IN
;              THE PROGRAM COUNTER SAVE LOCATION.
;
; The timing of this routine is critical for the correct operation of
; the SINGLE STEP and TRACE routines. The time required to execute this 
; routine is 187 CPU states (clock cycles) plus 10 CPU states for the JMP 
; instruction to jump to this routine immediately after starting the timer.
; When SINGLE STEP or TRACE commands are executed, the timer in 8155 
; is programmed to interrupt at 195 CPU states. This allows only one user 
; program instruction to be executed before the TRAP interrupt service 
; routine takes control.
; If any modifications are made to this routine, then CPU states must 
; recalculated again and the timer value must be adjusted at the same number.
;
;
; ***** THIS JS ALS0 THE ENTRY POINT FOR THE CRT *****
; *****      MONITOR TO RESTORE REGISTERS.       *****

RSTOR:
    LDA ISAV         ; Get user interrupt mask.
    ORI 18H         ; Enable setting of interrupt mask and
                     ; reset RST7.5 flip flop.
    SIM              ; Restore user interrupt mask.
                     ; Restore user interrupt status.
    LDA ISAV         ; Get user interrupt mask.
    ANI 08H          ; Should user interrupts be enabled ?
    JZ RSR05         ; No - Leave interrupts disabled.
    EI               ; Yes - Enable interrupts for user program.
    JMP RSR10
RSR05:
    STC              ; Two dummy instructions - When single step or trace
    JNC RSR10        ; routine is being used, the timer is
                     ; running and the execution time for this 
                     ; routine must not vary.
RSR10:
    LXI H,USRRG      ; Set monitor stack pointer to start of
    SPHL             ; user register save area.
    POP D            ; Restore registers.
    POP B
    POP PSW
    LHLD SSAV        ; Restore user stack pointer.
    SPHL
    LHLD PSAV
    PUSH H           ; Put user program counter on stack.
    LHLD LSAV        ; Restore H,L registers.
    RET              ; Jump to user program counter and execute
; the instruction(s). If this subroutine is 
; called from single step or trace, only one 
; instruction will be executed, because an 
; interrupt from the timer will occure during 
; execution of this instruction.

;***************************************************************************

; FUNCTION   : TRAP_SERV - INTERRUPT DRIVEN ROUTINE - TRAP SERVICE.
; INPUTS     : NONE 
; OUTPUTS    : NONE 
; CALLS      : NONE 
; DESTROYS   : A, F/F
; DESCRIPTION: TRAP_SERV JUMPS TO PROPER INTERRUPT SERVICE ROUTINE 
;              DEPENDING ON THE MONITOR USED.


TRAP_SERV:
    PUSH PSW         ; Save PSW for later use.
    IN MONSEL        ; Find which monitor is used.
                     ; Keyboard/Display monitor or CRT monitor ?
    ANI MASK0        ; Mask off all bits exept 0.
    JZ TRACE6        ; If bitO = 0 go to CRT monitor TRAP service routine.
    JMP STP25        ; If it is 1 go to Keyboard TRAP service routine.

;***************************************************************************    

; FUNCTION   : UPDAD - UPDATE ADDRESS FIELD OF DISPLAY
; INPUTS     : B -DOT FLAG - 1 MEANS PUT DOT AT RIGHT EDGE OF FIELD
;                          - 0 MEANS NO DOT
; OUTPUTS    : NONE 
; CALLS      : HXDSP,OUTPT 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: UPDAD UPDATES THE ADDRESS FIELD OF THE DISPLAY 
;              USING THE DATA IN CURRENT ADDRESS. THE TWO BYTES
;              OF CURRENT ADDRESS ARE EXPANDED TO FOUR HEX DIGITS.

UPDAD:
    LHLD CURAD       ; Get current address.
    XCHG             ; ARG - Put current address in D,E.
    CALL HXDSP       ; Expand current address for display.
                     ; ARG - Address of expanded address is in H,L.
    MVI A,ADFLD      ; ARG - Use address field of display.
                     ; ARG - Dot flag is in B.
    CALL OUTPT       ; Output current address to address field.
    RET

;*************************************************************************** 

; FUNCTION: UPDDT - UPDATE DATA FIELD OF DISPLAY 
; INPUTS     : B - NOT FLAG - 1 MEANS PUT DOT AT RIGHT EDGE OF FIELD 
;                           - 0 MEANS NO DOT
; OUTPUTS    : NONE 
; CALLS      : HXDSP,OUTDT 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: UPDDT UPDATES THE DATA FIELD OF THE DISPLAY USING 
;              THE CURRENT DATA BYTE. THE BYTE IN CURRENT DATA IS
;              EXPANDED TO TWO HEX DIGITS.

UPDDT:
    LDA CURDT        ; Get current data.
    MOV D,A          ; ARG - Put current data in D.
    CALL HXDSP       ; Expand current data for display.
                     ; ARG - Address of expanded data is in H,L.
    MVI A,DTFLD      ; ARG - Use data field of display.
                     ; ARG - Dot flag is in B.
    CALL OUTPT       ; Output current data to data field.
    RET

;***************************************************************************
;
; KEYBOARD/DI SPLAY MONITOR TABLES
;
;***************************************************************************

;       COMMAND TABLE
;       =============
; CODED COMMAND CHARACTERS AS RECEIVED FROM KEYBOARD.

CMDTB:
    DB 12H            ; GO command.
    DB 13H            ; SUBSTITUTE MEMORY command.
    DB 14H            ; EXAMINE REGISTERS command.
    DB 15H            ; SINGLE STEP command.
NUMC EQU $-CMDTB      ; Number of commands.

;***************************************************************************

; COMMAND ROUTINE ADDRESS TABLE.
; (MUST BE IN REVERSE ORDER OF COMMAND TABLE).

CMDAD:
    DW SSTEP         ; Address of SINGLE STEP routine.
    DW EXAM          ; Address of EXAMINE REGISTERS routine.
    DW SUBST         ; Address of SUBSTITUTE MEMORY routine.
    DW GOCMD         ; Address of GO routine.
    
;***************************************************************************

DSPTB:               ; Table for translating characters for output.

;             DISPLAY
;             FORMAT   CHARACTER
;             ======   =========

ZERO EQU $-DSPTB
    DB 0F3H          ; 0
    DB 60H           ; 1
    DB 0B5H          ; 2
    DB 0F4H          ; 3
    DB 66H           ; 4
FIVE EQU $-DSPTB
LETRS EQU $-DSPTB
    DB 0D6H          ; 5 and S
    DB 0D7H          ; 6
    DB 70H           ; 7
EIGHT EQU $-DSPTB
    DB 0F7H          ; 8
    DB 76H           ; 9
LETRA EQU $-DSPTB
    DB 77H           ; A
LETRB EQU $-DSPTB
    DB 0C7H          ; b
LETRC EQU $-DSPTB
    DB 93H           ; C
LETRD EQU $-DSPTB
    DB 0E5H          ; d
LETRE EQU $-DSPTB
    DB 97H           ; E
LETRF EQU $-DSPTB
    DB 17H           ; F
LETRH EQU $-DSPTB
    DB 67H           ; H
LETRL EQU $-DSPTB
    DB 83H           ; L
LETRP EQU $-DSPTB
    DB 37H           ; P
LETRI EQU $-DSPTB
    DB 60H           ; I
LETRR EQU $-DSPTB
    DB 05H           ; r
BLANK EQU $-DSPTB
    DB 00H           ; Blank
LETRT EQU $-DSPTB
    DB 87H           ; t

;***************************************************************************

; KEYBOARD/DI SPLAY MONITOR MESSAGES TO MICROKIT'S DISPLAY

BLNKS: DB BLANK,BLANK,BLANK,BLANK ; For address or data field.
ERMSG: DB BLANK,LETRE,LETRR,LETRR ; Error message for address field.
EXMSG: DB LETRE,BLANK,BLANK,BLANK ; Execution message for address field.
SGNAD: DB BLANK,BLANK,EIGHT,ZERO  ; Sign-on message at address field.
SGNDT: DB EIGHT,FIVE              ; Sign-on message at data field.

;****************************************************************************

NMTBL:                            ; Register name table.
                                  ; Names of registers in display format.
    DB BLANK,BLANK,BLANK,LETRA    ; A register.
    DB BLANK,BLANK,BLANK,LETRB    ; B register.
    DB BLANK,BLANK,BLANK,LETRC    ; C register.
    DB BLANK,BLANK,BLANK,LETRD    ; D register.
    DB BLANK,BLANK,BLANK,LETRE    ; E register.
    DB BLANK,BLANK,BLANK,LETRF    ; F register.
    DB BLANK,BLANK,BLANK,LETRI    ; Interrupt mask.
    DB BLANK,BLANK,BLANK,LETRH    ; H register.
    DB BLANK,BLANK,BLANK,LETRL    ; L register.
    DB BLANK,LETRS,LETRP,LETRH    ; Stack pointer high order byte.
    DB BLANK,LETRS,LETRP,LETRL    ; Stack pointer low order byte.
    DB BLANK,LETRP,LETRC,LETRH    ; Program counter high byte.
    DB BLANK,LETRP,LETRC,LETRL    ; Program counter low byte.

;***************************************************************************

; USER REGISTER SAVE LOCATION TABLE.
; THIS TABLE IS AN ADDRESS TABLE OF USER REGISTERS SAVE LOCATIONS.
; ONLY THE LOW BYTE IS USED BY THIS TABLE.
; THE HIGH BYTE IS ALWAYS 40H (MEANS THE 4000H RAM POSITION)

RGTBL:
    DB (ASAV & 0FFH )            ; A register.
    DB (BSAV & 0FFH )            ; B register.
    DB (CSAV & 0FFH )            ; C register.
    DB (DSAV & 0FFH )            ; D register.
    DB (ESAV & 0FFH )            ; E register.
    DB (FSAV & 0FFH )            ; Flags.
    DB (ISAV & 0FFH )            ; Interrupt mask.
    DB (HSAV & 0FFH )            ; H register.
    DB (LSAV & 0FFH )            ; L register.
    DB (SPHSV & 0FFH )           ; Stack pointer high order byte.
    DB (SPLSV & 0FFH )           ; Stack pointer low order byte.
    DB (PCHSV & 0FFH )           ; Program counter high order byte.
    DB (PCLSV & 0FFH )           ; Program counter low order byte.
NUMRG EQU ($-RGTBL)               ; Number of entries in save location table.

;    END OF KEYBOARD MONITOR.


;***************************************************************************
;***************************************************************************
;
; CRT MONITOR
;
;***************************************************************************
;***************************************************************************

; ABSTRACT
; ========
; THIS PROGRAM RUNS ON THE MICROKIT BOARD AND IS DESIGNED TO 
; PROVIDE THE USER WITH A MINIMAL MONITOR. BY USING THIS PROGRAM 
; THE USER CAN EXAMINE AND CHANGE MEMORY OR CPU REGISTERS, LOAD 
; A PROGRAM (IN ABSOLUTE HEX) INTO RAM, AND EXECUTE INSTRUCTIONS 
; ALREADY IN MEMORY. THE MONITOR PROVIDES THE USER ALL THE 
; ROUTINES FOR PERFORMING CONSOLE I/O.

; PROGRAM ORGANIZATION
; ====================
; THE MONITOR IS ORGANIZED IN THE FOLLOWING WAY. FIRST THE 
; COMMAND RECOGNIZER, WHICH IS THE HIGHEST LEVEL ROUTINE IN THE 
; PROGRAM. NEXT THE ROUTINES TO IMPLEMENT THE VARIOUS COMMANDS.
; FINALLY, THE UTILITY ROUTINES WHICH ACTUALLY DO THE DIRTY WORK.

; LIST OF FUNCTIONS
; =================
;   GETCM            : COMMAND RECOGNIZER
;   -----
;   DISPL_CMD        : DISPLAY MEMORY
;   ENTER_CMD        : ENTER HEX DIGITS TO MEMORY 
;   FILL_CMD         : FILL MEMORY BLOCK WITH DATA
;   GO_CMD           : USER PROGRAM EXECUTION
;   HELP_CMD         : DISPLAY HELP SCREENS
;   IN_CMD           : INPUT DATA FROM PORT
;   MOVE_CMD         : MOVE DATA IN MEMORY
;   OUT_CMD          : OUTPUT DATA TO PORT
;   PORT_CMD         : SELECT ACTIVE PORT FOR DOWNLOAD
;   SUBST_CMD        : SUBSTITUTE MEMORY CONTENTS
;   TRACE_CMD        : SINGLE STEP
;   XREG_CMD         : EXAMINE AND MODIFY CPU REGISTERS
;   WRITE_CMD        ; WRITE DATA TO TAPE
;   READ_CMD         ; READ DATA FROM TAPE
;   LOAD_CMD         ; DOWNLOAD A HEX FILE COMMAND
;   -----
;   CI               : CONSOLE INPUT
;   CO               : CONSOLE OUTPUT
;   CROUT            : SEND CARRIAGE RETURN TO CONSOLE
;   DELAY            : SOFTWARE DELAY
;   DISPL_MES        : DISPLAY A MESSAGE TO CRT
;   DISP_PC          : DISPLAY USER PROGRAM COUNTER AND INSTRUCTION CODE
;   ECHO             : ECHO CHARACTERS TO CONSOLE
;   ERROR            : SEND ERROR MESSAGE TO CONSOLE
;   FRET             : RETURN FALSE
;   GETCH            : GET A CHARACTER
;   GETHX            : GET HEX CHARACTERS
;   GETNM            : GET COUNT OF NUMBERS
;   HILO             : COMPARE HL WITH DE
;   HEX_BIN          : CONVERT HEX ASCII TO BINARY
;   LDELAY           : LONG SOFTWARE DELAY
;   NMOUT            : CONVERT 8 BITS INTO TWO ASCII
;   PRVAL            : CONVERT 4 BITS INTO ONE ASCII
;   REGDS            : DISPLAY CONTENTS OF SAVED REGISTERS
;   RGADR            : FIND REGISTER ADDRESS SAVE LOCATION
;   SRET             : RETURN TRUE
;   STHFO            : CHECK HALF BYTE FLAG
;   STHLF            : STORE 4 BITS IN HALF BYTE
;   VALDG            : CHECK FOR VALID HEX DIGIT
;   VALDL            : CHECK FOR VALID DELIMITER

;*************************************************************************** 
; BEGINNING OF CRT MONITOR.
; DISPLAY SIGN-ON MESSAGE AT CRT.
; WHEN YOU ARE IN THE CRT MONITOR THE MICROKIT'S DISPLAY SHOWS 
; THE LETTERS "Crt" AT THE ADDRESS FIELD.

CRTMON:
    MVI A,ADFLD      ; Use address field of display.
    MVI B,NODOT      ; No dot on display.
    LXI H,CONAD      ; Use address field to display 'Crt1.
    CALL OUTPT       ; Write 'Crt' on the display.
    MVI A,DTFLD      ; Use data field of display.
    MVI B,NODOT      ; No dot on display.
    LXI H,CONDAT     ; Display blanks.
    CALL OUTPT       ; Blank data field.
    MVI A,EMPTY      ; No characters are received from keyboard yet.
    STA IBUFF
    LXI H,SIGN_ON    ; Get address of sign-on massage.
    CALL DISPL_MES   ; Display sign-on message.

;***************************************************************************
;
; CRT MONITOR COMMAND RECOGNIZING ROUTINE
;
;***************************************************************************

; FUNCTION   : GETCM 
; INPUTS     : NONE 
; OUTPUTS    : NONE 
; CALLS      : GETCH,ECHO,ERROR 
; DESTROYS   : A,B,C,H,L,F/F
;
; DESCRIPTION: GETCM RECEIVES AN INPUT CHARACTER FROM THE USER 
;              AND ATTEMPTS TO LOCATE THIS CHARACTER IN ITS
;              COMMAND CHARACTER TABLE. IF SUCCESFUL, THE ROUTINE
;              CORRESPONDING TO THIS CHARACTER IS SELECTED FROM
;              A TABLE OF COMMAND ROUTINE ADDRESSES, AND CONTROL
;              IS TRANSFERRED TO THIS ROUTINE. IF THE CHARACTER
;              DOES NOT MATCH ANY ENTRIES, CONTROL IS PASSED TO
;              THE ERROR HANDLER.

GETCM:
    LXI H,MNSTK-2    ; Always want to reset stack pointer to monitor
    SPHL             ; starting value so routines needn't clean up.
    MVI C,'.'        ; Prompt character to register C.
    CALL ECHO        ; Send prompt character to user terminal.
    CALL GETCH       ; Get command character to register C.
    CALL ECHO        ; Echo character to user terminal.
    MOV A,C          ; Put command character into register A.
    LXI B,NCMDS      ; Register C contains loop and index count.
    LXI H,CTAB       ; HL points into command table.
GTC05:
    CMP M            ; Compare table entry and character.
    JZ GTC10         ; Branch if equal - Command recognized.
    INX H            ; Else, increment table pointer.
    DCR C            ; Decrement loop count.
    JNZ GTC05        ; Branch if not at table end.
    JMP ERROR        ; Else, command character is illegal.
GTC10:
    LXI H,CADR       ; Command recognized - Load address of table
                     ; of command routine addresses.
    DAD B            ; Add what is left of loop count.
    DAD B            ; Add again - Each entry in table "CADR" is 2 bytes.
    MOV A,M          ; Get low byte of address of table entry to reg. A.
    INX H            ; Point to next byte in table.
    MOV H,M          ; Get high byte of address of table entry to reg. H.
    MOV L,A          ; Put low byte of table entry into register L.
    PCHL             ; Next instruction comes from command routine.
                     ; Jump to proper command implemending routine.

;***************************************************************************
;
; CRT MONITOR COMMAND IMPLEMENDING ROUTINES
;
;***************************************************************************

; FUNCTION   : DISPL_CMD 
; INPUTS     : NONE 
; OUTPUTS    : NONE
; CALLS      : ECHO,NMOUT,HILO,GETCM,GETNM,CROUT 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: DISPL_CMD IMPLEMENTS THE DISPLAY MEMORY (D) COMMAND

DISPL_CMD:
    MVI C,2          ; Get two 16 bit numbers from input stream.
    CALL GETNM
    POP D            ; Ending address to DE.
    POP H            ; Starting address to HL.
DCM05:
    CALL CROUT       ; Echo carriage return/line feed.
    MOV A,H          ; Display high byte address of first location in line.
    CALL NMOUT
    MOV A,L          ; Display the low byte addr. of first location in line.
    CALL NMOUT
DCM10:
    MVI C,' '
    CALL ECHO        ; Use blank (space) as seperation.
    MOV A,M          ; Get the contents of memory location.
    CALL NMOUT       ; Display contents.
    CALL HILO        ; See if address of displayed location is greater
                     ; than or equal to ending address.
    JC DCM15         ; No carry means that there are more data to display.
    CALL CROUT       ; Carriage return/line feed to end line.
    JMP GETCM        ; All done.
DCM15:
    INX H            ; Point to next memory location for display.
    MOV A,L          ; Get low order bits of new address.
    ANI NEWLN        ; See if last hex digit of address
                     ; denotes start of new line.
    JNZ DCM10        ; No - Not at end of line.
    JMP DCM05        ; Yes - Start new line with address.

;****************************************************************************

; FUNCTION   : ENTER_CMD 
; INPUTS     : NONE 
; OUTPUTS    : NONE
; CALLS      : ERROR,ECHO,GETCH,VALDG,HEX_BIN,STHLF,GETNM,CROUT,VALDL,STHFO,GETCM 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: ENTER_CMD IMPLEMENTS THE ENTER HEX DIGITS INTO MEMORY.
;              THIS IS THE (E) COMMAND.

ENTER_CMD:
    MVI C,1
    CALL GETNM       ; Get single number from input stream.
    MVI A,UPPER
    STA TEMP         ; Temp will hold the upper/lower half byte flag.
    POP D            ; Address of start to DE.
ENTR05:
    CALL GETCH       ; Get a character from input stream.
    MOV C,A
    CALL ECHO        ; Echo the character to console.
    MOV A,C          ; Put character back into A.
    CPI ESCAPE       ; See if character is a terminating character (ESC).
    JZ ENTR25        ; If so, all done entering characters.
    CALL VALDL       ; Else, see if valid delimiter.
    JC ENTR05        ; If the character is a terminator ignore it.
    CALL VALDG       ; Else, check to see if valid hex digit.
    JNC ENTR20       ; If not, branch to handle error condition.
    MOV A,C          ; Put character in register A.
    CALL HEX_BIN     ; Convert digit to binary.
    MOV C,A          ; Move result to register C.
    CALL STHLF       ; Store in appropriate half word.
    LDA TEMP         ; Get half byte flag.
    ORA A            ; Set F/F.
    JNZ ENTR10       ; Branch if flag set for upper.
    INX D            ; If lower, increment address of byte to store in.
ENTR10:
    XRI INVRT        ; Toggle state of flag.
    STA TEMP         ; Put new value of flag back.
    JMP ENTR05       ; Process next digit.
ENTR20:
    CALL STHFO       ; Illegal character. Make sure entire byte filled.
    JMP ERROR        ; Jump to error.
ENTR25:
    CALL STHFO       ; Input is done. Come here with ESC character.
    CALL CROUT       ; Add carriage return.
    JMP GETCM

;****************************************************************************

; FUNCTION   : FILL_CMD 
; INPUTS     : NONE 
; OUTPUTS    : NONE 
; CALLS      : GETNM, HILO, GETCM 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: FILL_CMD IMPLEMENTS THE 'FILL MEMORY' COMMAND.
;              USER SPECIFIES A MEMORY BLOCK AND THE DATA TO BE FILLED UP.

FILL_CMD:
    MVI C,3
    CALL GETNM       ; Get three 16 bit numbers from input stream.
    POP B            ; Put fill byte in C.
    POP D            ; Put ending address in DE.
    POP H            ; Put starting address in HL.
    CALL HILO        ; See if Ending address > Source address.
    JNZ FIL05        ; DE > HL - Continue.
    JMP ERROR        ; Ending address < Source address.
    
FIL05:
    MOV M,C          ; Fill memory with byte.
    MOV A,M          ; Read back the byte j'ust written.
    CMP C            ; Byte written o.k. ?
    JNZ ERROR        ; If not the same goto error. Can't write to
                     ; EPROM or non existing RAM.
    INX H            ; Increment address.
    CALL HILO        ; See if ending address £ source address.
    JC FIL05         ; If carry = 1 command is done.
    MOV M,C          ; Fill the last memory location.
    JMP GETCM        ; All done - Get next command.

;****************************************************************************

; FUNCTION   : GO_CMD 
; INPUTS     : NONE 
; OUTPUTS    : NONE 
; CALLS      : ERROR,GETHX,RSTOR 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: GO_CMD IMPLEMENTS THE EXECUTE USER PROGRAM (G) COMMAND

GO_CMD:
    CALL DISP_PC     ; Display user's program counter.
    CALL GETHX       ; Get address (if present) from input stream.
    JNC GCM05        ; Branch if no number present.
    MOV A,D          ; Else, get terminal character.
    CPI CR           ; See if it is carriage return.
    JNZ ERROR        ; Error if the command is not properly terminated.
    LXI H,PSAV       ; Save the input hex number in place of user
    MOV M,C          ; program counter. This is the starting
    INX H            ; address of the program to be executed.
    MOV M,B 
    JMP GCM10
GCM05:
    MOV A,D          ; If no starting address, make sure that
    CPI CR           ; carriage return terminated command.
    JNZ ERROR        ; Error if it is not terminated by a "CR".
GCM10:
    JMP RSTOR        ; Restore registers and begin execution
                     ; (** RSTOR is in keyboard monitor **)
                     
;****************************************************************************

; FUNCTION   : HELP_CMD 
; INPUTS     : NONE 
; OUTPUTS    : NONE 
; CALLS      : DISPL_MES,CI 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: HELP_CMD IMPLEMENTS THE DISPLAY HELP SCREENS 

HELP_CMD:
    LXI H,HELP_SCREEN_1 ; Display HELP SCREEN 1 
    CALL DISPL_MES
    CALL CI
    CPI ESCAPE
    JZ EXIT_HELP
    LXI H,HELP_SCREEN_2 ; Display HELP SCREEN 2
    CALL DISPL_MES
EXIT_HELP:
    JMP GETCM           ; All done.

;****************************************************************************

; FUNCTION   : IN_CMD 
; INPUTS     : DATA FROM INPUT PORT 
; OUTPUTS    : NONE 
; CALLS      : GETHX,NMOUT,ECHO 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: IN_CMD IMPLEMENTS DATA INPUT FROM A PORT 

IN_CMD:
    CALL GETHX       ; Get input port address.
    MOV A,C          ; Port address in register BC. Use only reg. C.
    STA PORT+1       ; Put it in RAM.
    MVI A,0DBH       ; Code for IN instruction.
    STA PORT         ; Put it in RAM.
    MVI A,0C9H       ; Code for RET instruction.
    STA PORT+2       ; Put it in RAM.
IN05:
    MOV A,D          ; Get delimiter character.
    CPI ' '          ; Is it space ?
    JZ IN10          ; Yes - Go on.
    CPI ','          ; Is it comma ?
    JNZ GETCM        ; If none then must be CR.
IN10:
    CALL PORT        ; Perform an input instruction.
    CALL NMOUT       ; Display input data.
    CALL CROUT       ; Display a CR-LF.
    CALL GETHX       ; Get delimeter from console.
    JNC IN05         ; If it is valid repeat.
    JMP ERROR        ; If not valid delimeter - Error.
    
;****************************************************************************

; FUNCTION    : LOAD_CMD - DOWNLOAD MAIN PROGRAM 
; INPUTS      : NONE 
; OTPUTS      : NONE 
; CALLS       : DISPLMES, GETFL 
; DESTROYS    : A,B,C,D,E,H,L,F/F
; DESCRIPTION : LOAD_CMD IMPLEMENTS THE LOADING OF A HEX FILE.
;               IT LOADS THE CONTENTS OF THE FILE IN MICROKIT'S
;               RAM AND GIVES CONTROL TO CRT MONITOR PROGRAM.

LOAD_CMD:
    LXI H,LOAD_MES1      ; Address of 'LOADING A HEX FILE FROM' message.
    CALL DISPL_MES       ; Display the message.
    
    LDA DPORT            ; Get active port number.
    ADI 30H              ; Convert to ASCII.
    MOV C,A
    CALL ECHO            ; Display to CRT.
    
    LXI H,LOAD_MES2      ; Address of 'PLEASE WAIT message.
    CALL DISPL_MES       ; Display the message.
    
    CALL GETFL           ; Start downloading.
    INR A                ; is there a transmission error code ?
    JZ LOAD_ERR          ; Yes - Display error message.
    LXI H,LOAD_END_MES   ; No - Get address of 'TRANSMISSION END' message
    CALL DISPL_MES       ; Display the message.
    JMP GETCM            ; Exit to CRT monitor.
LOAD_ERR:
    LXI H,LOAD_ERROR_MES ; Address of 'TRANSMISSION ERROR' message.
    CALL DISPL_MES       ; Display the message.
    JMP GETCM            ; Exit to CRT monitor.

;****************************************************************************
    
; FUNCTION   : MOVE_CMD 
; INPUTS     : NONE 
; OUTPUTS    : NONE 
; CALLS      : GETCM,HILO,GETNM 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: MOVE_CMD IMPLEMENTS THE MOVE DATA IN MEMORY (M) COMMAND

MOVE_CMD:
    MVI C,3
    CALL GETNM       ; Get three 16 bit numbers from input stream.
    POP B            ; Destination address to BC.
    POP H            ; Ending address to HL.
    POP D            ; Starting address to DE.
    CALL HILO        ; See if Ending address > Starting address.
    JNZ MCM05        ; HL > DE - Continue.
    JMP ERROR        ; Ending address < Starting address.
MCM05:
    PUSH H           ; Save ending address.
    MOV H,D
    MOV L,E          ; Source address to HL.
    MOV A,M          ; Get source byte.
    MOV H,B
    MOV L,C          ; Destination address to HL.
    MOV M,A          ; Move byte to destination.
    CMP M            ; Byte written o.k. ?
    JNZ ERROR        ; If not the same goto error. Can't write to
                     ; EPROM or non existing RAM.
    INX B            ; Increment destination address.
    MOV A,B
    ORA C            ; Test for destination address overflow.
    JZ GETCM         ; If so, terminate command.
    INX D            ; Increment source address.
    POP H            ; Get ending address.
    CALL HILO        ; See if ending address > source address.
    JC GETCM         ; If not, command is done.
    JMP MCM05        ; Move another byte.

;***************************************************************************

; FUNCTION   : OUT_CMD 
; INPUTS     : DATA BYTE FROM CONSOLE 
; OUTPUTS    : DATA TO OUTPUT PORT 
; CALLS      : GETHX,ECHO,ERROR 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: OUTCMD IMPLEMENTS DATA OUTPUT TO A PORT 

OUT_CMD:
    CALL GETHX       ; Get output port.
    MOV A,D          ; Get delimeter.
    CPI ' '          ; Is it a space ?
    JZ OUT1          ; Yes - Go on.
    CPI ','          ; Is it a comma ?
    JNZ ERROR        ; No - Error.
OUT1:
    MOV A,C          ; Output port in register A.
    STA PORT+1       ; Put it in RAM.
    MVI A,0D3H       ; Code for OUT instruction.
    STA PORT         ; Put it in RAM.
    MVI A,0C9H       ; Code for RET instruction.
    STA PORT+2       ; Put it in RAM.
    CALL GETHX       ; Get data from user.
    MOV A,D          ; See if command terminated with a CR.
    CPI CR
    JNZ ERROR        ; No - Error.
    MOV A,C          ; Put data in register A.
    CALL PORT        ; Output data to port.
    JMP GETCM        ; Get next command.
    
;***************************************************************************

; FUNCTION   : PORT_CMD - SELECT DOWNLOAD PORT COMMAND 
; INPUTS     : NONE 
; OUTPUTS    : NONE
; CALLS      : DISPL_MES,CI,CROUT,ECHO 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: PORT_CMD IMPLEMENTS THE SELECT DOWNLOAD PORT COMMAND 

PORT_CMD:
    LXI H,DPORT_MES1 ; Display 'ACTIVE PORT IS DB9-' message.
    CALL DISPL_MES
    
    LDA DPORT        ; Get active download port.
    ADI 30H          ; Convrert to ASCII.
    MOV C,A
    CALL ECHO        ; Display it.
    
    LXI H,DPORT_MES2 ; Display 'PRESS 1,2 TO SELECT' message.
    CALL DISPL_MES
    
    CALL CI          ; Get user input.
    STA TEMP         ; Save input character.
    MOV C,A          ; Display input character to CRT.
    CALL ECHO
    LDA TEMP         ; Get input character.
    CPI '1'          ; Is it 1 ?
    JZ SET_PORT      ; Yes - Set active port to 1.
    CPI '2'          ; Is it 2 ?
    JZ SET_PORT      ; Yes - Set active port to 2.
    CPI CR           ; Is it CR ?
    JZ EXIT_PORT     ; Yes - Exit without altering.
    JMP ERROR        ; If user input none of the above error.
SET_PORT:
    SUI 30H          ; Convert to binary.
    STA DPORT        ; Save in RAM.
EXIT_PORT:
    CALL CROUT       ; Display CR-LF.
    JMP GETCM        ; Get next command.
    
;***************************************************************************

; FUNCTION: READ_CMD - READ DATA FROM TAPE 
; INPUTS : NONE 
; OUTPUTS : NONE
; CALLS : DISPL_MES,Cl,VALDL,READ,D_STR_END_ADDR,CROUT 
; DESTROYS: A,B,C,H,L,F/F
; DESCRIPTION: READ A FILE FROM TAPE. THE ROUTINE READS THE START,
;              END ADDRESSES AND THE DATA BYTES.
;              ADDS ALL DATA BYTES COMPARING THE RESULT WITH THE
;              CHECKSUM URITTEN ON TAPE. IF THE TWO CHECKSUMS ARE
;              THE SAME DISPLAYS THE START AND END ADDRESSES
;              AND GIVES CONTROL TO CRT monitor, else gives an
;              ERROR MESSAGE.

READ_CMD:
    MVI A,UNMSK
    SIM                 ; Unmask interrupts from the keyboard.
    
    LXI H,MES5
    CALL DISPL_MES      ; Display message "START TAPE RECORDER PRESS RETURN" .
    CALL CI             ; Wait for the user's answer.
    MOV C,A
    CALL VALDL          ; Is it a valid delimiter ?
    JC NOERR5           ; Yes - Continue.
    JMP ERROR           ; No - Error.
NOERR5:
    LXI H,MES8
    CALL DISPL_MES      ; Display message "READING FROM TAPE".
    CALL READ           ; Read the file from tape.
    JC NOERR6           ; File read o.k. ?
    LXI H,MES9          ; No - Checksum is not o.k.
    CALL DISPL_MES      ; Display message " READING ERROR "
    JMP GETOUT          ; Exit.
    
NOERR6:
    CALL D_STR_END_ADDR ; File read o.k. - Display start, end addresses.
GETOUT:
    CALL CROUT
    JMP GETCM

;***************************************************************************

; FUNCTION   : SUBST_CMD 
; INPUTS     : NONE 
; OUTPUTS    : NONE
; CALLS      : GETHX,GETCM,NMOUT,ECHO 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: SUBST_CMD IMPLEMENTS THE SUBSTITUTE MEMORY (S) COMMAND. 

SUBST_CMD:
    CALL GETHX       ; Get a 16 bit number from user.
    PUSH B           ; Move the 16 bit number in BC to HL.
    POP H            ; Get number to HL - Denotes memory location.
SCM05:
    MOV A,D          ; Get terminator.
    CPI ' '          ; See if space.
    JZ SCM10         ; Yes - Continue processing.
    CPI ','          ; See if comma.
    JNZ GETCM        ; No - Terminate command.
SCM10:
    MOV A,M          ; Get contens of specified location to register A.
    CALL NMOUT       ; Display contents on console.
    MVI C,'-'
    CALL ECHO        ; Use dash for separator.
    CALL GETHX       ; Get new value for memory location, if any.
    JNC SCM15        ; If no value present, don't change the data of
                     ; present memory location.
    MOV M,C          ; else, store lower 8 bits of new byte entered
                     ; by the user to present memory location.
SCM15:
    INX H            ; Increment address of memory location to view.
    JMP SCM05

;***************************************************************************

; FUNCTION   : TRACE_CMD 
; INPUTS     : NONE 
; OUTPUTS    : NONE 
; CALLS      : ERROR,CROUT,GETHX 
; DESTROYS   : A,B,C,D,H,L,F/F
; DESCRIPTION: TRACE_CMD IMPLEMENTS THE TRACE (T) COMMAND, (CRT SINGLE STEP). 

TRACE_CMD:
    CALL DISP_PC     ; Display user's program counter.
    CALL GETHX       ; Get user input.
    JNC TRACE1       ; Was the first input character a delimeter ?
                     ; No - User has entered a new address for the PC.
    LXI H,PSAV       ; Get the address where the user's PC is stored.
    MOV M,C          ; Put entered hex value (new address for PC)
    INX H            ; to user's PC.
    MOV M,B
    
TRACE1:
    MOV A,D          ; Get the character which terminated GETHX.
    CPI ','          ; Was it a comma ?
    JZ TRACE2        ; Yes - Prepare for single step - Go set timer.
    CPI CR           ; No - Perhaps it was a CR.
    JZ TR_EXIT       ; Yes it was a CR - Terminate command.
    JMP ERROR        ; No - Character from keyboard was neither
  
  ; a comma nor a CR - Goto error.
TR_EXIT:
    CALL CROUT       ; Dispalay CR/LF.
    JMP GETCM        ; Get new command.
    
TRACE2:
    LDA ISAV         ; Get user interrupt mask.
    ANI 08H          ; Keep interrupt status.
    STA TEMP         ; Save user interrupt status.
    LHLD PSAV        ; Get user PC.
    MOV A,M          ; Get user instruction.
    CPI 0F3H         ; Is it a DI instruction ?
    JNZ TRACE3       ; No.
    XRA A            ; Yes - Reset user interrupt status.
    JMP TRACE4
TRACE3:
    CPI 0FBH         ; Is it a El instruction ?
    JNZ TRACE5       ; No.
    MVI A,08H        ; Yes - Set user interrupt status.
TRACE4:
    STA TEMP         ; Save new user interrupt status.
TRACE5:              ; Start timer - Jump to user program.
    MVI A,80H        ; High order bits of timer value logicaly
    OUT TIMERHI      ; OR'ed with timer mode.
    MVI A,TMR_COUNT  ; Low order bits of timer value (197 CPU states).
    OUT TIMERLO
    LDA USCSR        ; Get user image of what is in CSR.
    ORI TSTART       ; Set timer command bits to start timer.
    OUT CSR_8155     ; Start timer.
    JMP RSTOR        ; Restore user registers - Start executing user's
                     ; program - When the first instruction is executed 
                     ; an interrupt will occur if you are in Single 
                     ; Step or Trace command.
                     
                     
TRACE6:              ; TRAP service routine for CRT monitor.
                     ; Branch here when 8155 timer interrupts after 
                     ; one user instruction.
    LDA USCSR        ; Get user image of what's in CSR.
    ANI 3FH          ; Clear 2 high order bits.
    ORI TSTOP        ; Set timer stop bits.
    OUT CSR_8155     ; Stop timer.
    POP PSW          ; Retrieve PSW.
    SHLD LSAV        ; Save HL.
    POP H            ; Get user program counter from top of stack.
    SHLD PSAV        ; Save user PC.
    PUSH PSW         ; Get user PSW.
    POP H            ; Put it in HL.
    SHLD FSAV        ; Save Flip/Flops and register A (PSW).
    LXI H,0          ; Clear HL.
    DAD SP           ; Get user stack pointer.
    SHLD SSAV        ; Save user stack pointer.
    LXI H,BSAV+1     ; Set monitor stack pointer for
                     ; saving remaining user registers.
    SPHL
    PUSH B           ; Save BC.
    PUSH D           ; Save DE.
    RIM              ; Get user interrupt mask.
    ANI 87H          ; Keep interrupt mask bits and SID pin data.
    LXI H,TEMP       ; Get user interrupt status.
    ORA M            ; OR it into mask.
    STA ISAV         ; Save interrupt status and mask.
    MVI A,UNMSK      ; Unmask interrupts from the keyboard.
    SIM
    JMP TRACE_CMD    ; Go get ready for another instruction.

;**************************************************************************

; FUNCTION   : XREG_CMD 
; INPUTS     : NONE 
; OUTPUTS    : NONE
; CALLS      : GETCH,ECHO,REGDS,GETCM,RGADR,NMOUT,CROUT,GETHX 
; DESTROYS   : A,B,C,D,H,L,F/F
; DESCRIPTION: XREG_CMD IMPLEMENTS THE REGISTER EXAMINE AND MODIFY (X) 
;              COMMAND.

XREG_CMD:
    CALL GETCH       ; Get register indentifier.
    CALL ECHO        ; Echo it to console.
    MOV A,C          ; Move character to register A.
    CPI CR           ; Compare it with the carriage return.
    JNZ XCM05        ; Branch if not carriage return.
    CALL REGDS       ; Else display all register contents.
    JMP GETCM        ; Then terminate command.
XCM05:
    CALL RGADR       ; Convert identifier into RTAB table address.
    PUSH B
    POP H            ; Put pointer to register entry into HL.
    MVI C,' '
    CALL ECHO        ; Echo space to user.
    MOV A,C
    STA TEMP         ; Put space into TEMP as delimiter.
XCM10:
    LDA TEMP         ; Get terminator.
    CPI ' '          ; See if a blank.
    JZ XCM15         ; Yes - Go check pointer into TTABLE.
    CPI ','          ; No - See if comma.
    JNZ GETCM        ; No - Must be carriage return to end command.
XCM15:
    MOV A,M
    ORA A            ; Set F/F.
    JNZ XCM18        ; Branch if not end of table.
    CALL CROUT       ; Else, output carriage return line feed.
    JMP GETCM        ; Exit.
XCM18:
    PUSH H           ; Put pointer on stack.
    MOV E,M          ; Put byte of pointer into register E.
    MVI D,(MNSTK >> 8) ; Address of save location from table.
    INX H            ; Increment pointer.
    MOV B,M          ; Fetch length flag from table.
    PUSH D           ; Save address of register save location.
    PUSH D
    POP H            ; Move address to HL.
    PUSH B           ; Save length flag.
    MOV A,M          ; Get 8 bits of register from save location.
    CALL NMOUT       ; Display it.
    POP PSW          ; Get back length flag.
    PUSH PSW         ; Save it again.
    ORA A            ; Set F/F.
    JZ XCM20         ; If 8 bit register, nothing more to display.
    DCX H            ; Else, for 16 bit register, get lower 8 bits.
    MOV A,M          ; Fetch lower 8 bits.
    CALL NMOUT       ; Display them.
XCM20:
    MVI C,'-'
    CALL ECHO        ; Use dash as separator.
    CALL GETHX       ; See if there is a value to put into register.
    JNC XCM30        ; No - Go check for next register.
    MOV A,D
    STA TEMP         ; Else, Save the terminator for now.
    POP PSW          ; Get back length flag.
    POP H            ; Put address of save location into HL.
    ORA A            ; Set F/F.
    JZ XCM25         ; If 8 bit register, branch.
    MOV M,B          ; Save upper 8 bits.
    DCX H            ; Point to save location for low 8 bits.
XCM25:
    MOV M,C          ; Store all of 8 bit or lower 1/2 of 16 bit reg.
XCM27:
    LXI D,RTABS      ; Size of entry in RTAB table.
    POP H            ; Pointer into register table RTAB.
    DAD D            ; Add entry size to pointer.
    JMP XCM10        ; Continue with next register.
XCM30:
    MOV A,D          ; Get terminator.
    STA TEMP         ; Save in memory.
    POP D            ; clear stack of length flag and address
    POP D            ; of save location.
    JMP XCM27        ; Go increment register table pointer.

;**************************************************************************
    
; FUNCTION   : WRITE_CMD - WRITE DATA TO TAPE 
; INPUTS     : NONE
; OUTPUTS    : NONE
; CALLS      : ERROR,HILO,DISPL_MES,Cl,VALDL,WRITE,D_STR_END_ADDR
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: THIS ROUTINE WRITES DATA TO TAPE.
;              ASKS FOR THE START AND END ADDRESSES. THEN ASKS
;              THE USER TO START TAPE RECORDER AND WRITES THE DATA
;              BLOCK ON TAPE, DISPLAYS THE START AND END ADDRESSES
;              AND GIVES CONTROL TO CRT MONITOR.

WRITE_CMD:
    MVI A,UNMSK
    SIM              ; Unmask interrupts from the keyboard.  
    
    LXI H,MES1       ; Address for message " START ADDRESS : ".
    CALL DISPL_MES   ; Display the message.
    CALL GETHX       ; Get the address.
    MOV H,B          ; Save input in registers H,L.
    MOV L,C          ; Else save the address.
    JC NOERR1        ; Any hex digits entered ?
    LXI H,MES2       ; No - Address for message " NOT VALID ADDRESS
    CALL DISPL_MES
    JMP TWREND       ; Exit.
NOERR1:
    SHLD STADR       ; Yes - Save start address.
ENDADR:
    LXI H,MES3       ; Address for message " END ADDRESS : ".
    CALL DISPL_MES   ; Display the message.
    CALL GETHX       ; Get the address.
    MOV H,B          ; Save input in registers H,L.
    MOV L,C          ; Else save the address.
    JC NOERR2        ; Any hex digits entered ?
    LXI H,MES2       ; No - Display message 11 NOT VALID ADDRESS ".
    CALL DISPL_MES
    JMP TWREND       ; Exit.
NOERR2:
    SHLD EDADR       ; Save end address.
    XCHG
    LHLD STADR
    CALL HILO        ; Find if start address < end address .
    JC NOERR3
    LXI H,MES4       ; If not display message "START ADDRESS>END ADDRESS".
    CALL DISPL_MES
    JMP TWREND       ; Exit.
NOERR3:
    LXI H,MES5
    CALL DISPL_MES    ; Display message » START TAPE RECORDER PRESS RETURN"
    CALL CI          ; Wait for the answer.
    MOV C,A
    CALL VALDL       ; Is it a valid delimiter ?
    JNC ERROR        ; No - Error.
    LXI H,MES6       ; Yes - Go on.
    CALL DISPL_MES   ; Display message "WRITING ON TAPE".
    CALL WRITE       ; Write the data block on tape.
    CALL D_STR_END_ADDR ; Display start and end addresses.
    LXI H,MES7       ; Display message " END OF WRITING ".
    CALL DISPL_MES
TWREND:
    CALL CROUT       ; Display CR.
    JMP GETCM        ; Go and get next command.

;***************************************************************************
;
; CRT MONITOR UTILITY ROUTINES
;
;***************************************************************************

; FUNCTION   : CI 
; INPUTS     : NONE
; OUTPUTS    : A - CHARACTER FROM CRT
; CALLS      : NOTHING 
; DESTROYS   : A,F/F
; DESCRIPTION: CI WAITS UNTIL A CHARACTER HAS BEEN ENTERED AT THE 
;              CRT AND THEN RETURNS THE CHARACTER, VIA THE
;              REGISTER A, TO THE CALLING ROUTINE.

CI:
    IN UART_CTL_ST   ; Get status of console.
    ANI UART_RBR     ; Check for receiver buffer ready.
    JZ CI            ; Not ready yet - Wait.
    IN UART_DATA     ; Ready - So get console character in register A.
    ANI 7FH          ; Mask off bit 8.
    RET              ; Return to calling routine.
    
;*************************************************************************** 

; FUNCTION: HEX_BIN
; INPUTS  : A - ASCII CHARACTER "0"-"9" OR  "A"-"F"
; OUTPUTS : A - 0 TO F HEX 
; CALLS   : NOTHING 
; DESTROYS: A,F/F
; DESCRIPTION: HEX_BIN CONVERTS THE ASCII REPRESENTATION OF A HEX 
;              NUMBER TO ITS CORRESPONDING BINARY VALUE. HEX_BIN
;              DOES NOT CHECK THE VALIDITY OF ITS INPUT.

HEX_BIN:
    SUI '0'          ; Subtract code for 'O' from argument.
    CPI 10           ; Want to test for result of 0 to 9.
    RM               ; If so, then all done.
    SUI 7            ; Else, result between 17 and 23 decimal.
    RET              ; So return after subtracting bias of 7.
    
;*************************************************************************** 

; FUNCTION   : CO
; INPUTS     : C - CHARACTER TO BE SEND TO CONSOLE CRT 
; OUTPUTS    : C - CHARACTER OUTPUT TO CONSOLE CRT 
; CALLS      : NOTHING 
; DESTROYS   : A,F/F
; DESCRIPTION: CO SENDS THE INPUT ARGUMENT TO THE CONSOLE CRT.
;              IF AN XOFF CHARACTER HAS BEEN RECEIVED, THE
;              ROUTINE WAITS THE RECEPTION OF AN XON CHARACTER
;              AND THEN SENDS THE INPUT CHARACTER TO THE CONSOLE.

CO:
    IN UART_CTL_ST   ; Read console status.
    ANI UART_RBR     ; Check for receiver buffer ready.
    JZ COXON         ; No character received. Transmit.
    IN UART_DATA     ; A character is received. Read the character.
    CPI CTRL_C       ; Is it a Ctrl-C character.
    JZ ERROR         ; Yes - Character is a Ctrl-C - Stop and exit.
    CPI XOFF         ; Is it XOFF ?
    JNZ COXON        ; No, ignore input. Transmit.
COXOFF:
    IN UART_CTL_ST   ; Character is XOFF. Read console status.
    ANI UART_RBR     ; Check for receiver buffer ready.
    JZ COXOFF        ; No, wait until ready.
    
    IN UART_DATA     ; Read character.
    CPI XON          ; Is it XON ?
    JNZ COXOFF       ; No, wait for XON.
    
    
COXON:
    IN UART_CTL_ST   ; Read console status.
    ANI UART_TRDY    ; Check for transmiter buffer ready.
    JZ COXON         ; No, wait until ready.
    MOV A,C
    OUT UART_DATA    ; Send character to the console.
    RET
    
;***************************************************************************

; FUNCTION   : CROUT 
; INPUTS     : NONE 
; OUTPUTS    : NONE 
; CALLS      : ECHO 
; DESTROYS   : A,B,C,F/F
; DESCRIPTION: CROUT SENDS A CARRIAGE RETURN (AND HENCE A LINE 
;              FEED) TO THE CONSOLE.

CROUT:
    MVI C,CR
    CALL ECHO
    RET

;*************************************************************************** 

; FUNCTION   : DELAY
; INPUTS     : DE - 16 BIT INTEGER DENOTING NUMBER OF TIMES TO LOOP 
; OUTPUTS    : NONE 
; CALLS      : NOTHING 
; DESTROYS   : A,D,E,F/F
; DESCRIPTION: DELAY DOES NOT RETURN TO CALLER UNTIL ARGUMENT IS 
;              COUNTED DOWN TO 0.

DELAY:
    DCX D            ; Decrement input argument.
    MOV A,D          ; Move high byte to register A.
    ORA E            ; Compare high byte with low.
    JNZ DELAY        ; If argument not 0, keep going.
    RET
    
;***************************************************************************

; FUNCTION   : DISPL_MES - DISPLAY A MESSAGE TO CRT 
; INPUTS     : H,L THE START ADDRESS OF MESSAGE 
; OUTPUTS    : NONE 
; CALLS      : CO 
; DESTROYS   : A,8,C,H,L,F/F
; DESCRIPTION: DISPLAYS A MESSAGE TO SCREEN. THE FIRST CHARACTER 
;              OF THE MESSAGE IS POINTED BY H,L.
;              THE MESSAGE TERMINATES WITH A NULL (00).

DISPL_MES:
    MOV A,M          ; Get a character.
    CPI 00           ; If it is 00, all characters are
    RZ               ; displayed so return.
    MOV C,A
    CALL CO          ; Echo the character on screen.
    INX H
    JMP DISPL_MES    ; Get next character. 
    
;***************************************************************************

; FUNCTION   : DISP_PC 
; INPUTS     : NONE
; OUTPUTS    : USER'S PC TO TERMINAL 
; CALLS      : ECHO, NMOUT 
; DESTROYS   : A,C,H,L,F/F
; DESCRIPTION: DISP_PC DISPLAYS AT THE CRT TERMINAL THE USER'S PROGRAM 
;              COUNTER AND THE OPERATION CODE POINTED BY THE PROGRAM
;              COUNTER.

DISP_PC:
    CALL CROUT       ; Display CR/LF.
    MVI C,' '        ; Display a space.
    CALL ECHO
    MVI C,'P'        ; Display letter P.
    CALL ECHO
    MVI C,'C'        ; Display letter C.
    CALL ECHO
    MVI C,'='        ; Display =.
    CALL ECHO
    
    LHLD PSAV        ; Get current value of user's PC.
    MOV A,H
    CALL NMOUT       ; Display high byte of user's PC.
    
    LHLD PSAV
    MOV A,L
    CALL NMOUT       ; Display low byte of user's PC.
    MVI C,' '        ; Display a space.
    CALL ECHO
    LHLD PSAV        ; Get from memory the instruction code
    MOV A,M          ; which is pointed by the user's PC.
    CALL NMOUT       ; Display it.
    
    MVI C,' '        ; Display a space.
    CALL ECHO
    MVI C,'-'        ; Display a dash to indicate opening for new value.
    CALL ECHO
    RET

;****************************************************************************

; FUNCTION   : ECHO
; INPUTS     : C - CHARACTER TO BE ECHOED TO TERMINAL 
; OUTPUTS    : C - CHARACTER ECHOED TO TERMINAL 
; CALLS      : CO 
; DESTROYS   : A,B,F/F
; DESCRIPTION: ECHO TAKES A SINGLE CHARACTER AS INPUT AND 
;              SENDS THAT CHARACTER TO THE USER TERMINAL.
;              A CARRIAGE RETURN IS ECHOED AS A CARRIAGE RETURN
;              LINE FEED, AND AN ESCAPE CHARACTER IS ECHOED AS $.

ECHO:
    MOV B,C          ; Save argument.
    MVI A,ESCAPE
    CMP B            ; See if echoing an escape character.
    JNZ ECHO5        ; No - Branch.
    MVI C,'$'        ; Yes - Echo as $.
ECHO5:
    CALL CO          ; Output through monitor.
    MVI A,CR
    CMP B            ; See if character echoed was a carriage return.
    JNZ ECH10        ; No - No need to take special action.
    MVI C,LF         ; Yes - Want to echo line feed, as well.
    CALL CO
ECH10:
    MOV C,B          ; Restore argument.
    RET

;***************************************************************************

; FUNCTION   : ERROR 
; INPUTS     : NONE 
; OUTPUTS    : NONE 
; CALLS      : ECHO,CROUT,GETCM 
; DESTROYS   : A,B,C,F/F
; DESCRIPTION: ERROR PRINTS THE ERROR CHARACTER (AN ASTERISK *)
;              ON THE CONCOLE, FOLLOWED BY CARRIAGE RETURN - LINE FEED
;              AND THEN RETURNS CONTROL TO THE COMMAND RECOGNIZER.

ERROR:
    MVI C,'*'
    CALL ECHO        ; Display to concote.
    CALL CROUT       ; Display carriage return/line feed.
    JMP GETCM        ; Get another command.
    
;***************************************************************************

; FUNCTION   : FRET 
; INPUTS     : NONE 
; OUTPUTS    : CARRY - ALWAYS 0 
; CALLS      : NOTHING 
; DESTROYS   : CARRY
; DESCRIPTION: FRET IS JUMPED TO BY ANY ROUTINE THAT WISHES TO 
;              INDICATE FAILURE ON RETURN. FRET SETS THE CARRY
;              FALSE, DENOTING FAILURE, AND THEN RETURNS TO THE
;              CALLING ROUTINE.

FRET:
    STC              ; First set carry true.
    CMC              ; Then complement it to make it false.
    RET              ; Return appropriately.
    
;***************************************************************************

; FUNCTION   : GETCH 
; INPUTS     : NONE
; OUTPUTS    : C - NEXT CHARACTER IN INPUT STREAM 
; CALLS      : CI 
; DESTROYS   : A,C,F/F
; DESCRIPTION: GETCH RETURNS THE NEXT CHARACTER IN THE INPUT 
;              STREAM TO THE CALLING PROGRAM.

GETCH:
    CALL CI          ; Get character from terminal.
    ANI PRTY0        ; Turn off parity bit in case set by console.
    MOV C,A          ; Put value in register C for return.
    RET
    
;***************************************************************************

; FUNCTION   : GETHX
; INPUTS     : NONE
; OUTPUTS    : BC - 16 BIT INTEGER
;              D - CHARACTER WHICH TERMINATED THE ROUTINE
;              CARRY - 1 IF FIRST CHARACTER NOT DELIMITER
;                    - 0 IF FIRST CHARACTER IS DELIMITER
; CALLS      : GETCH,ECHO,VALDL,VALDG,HEX_BIN,ERROR 
; DESTROYS   : A,B,C,D,E,F/F
; DESCRIPTION: GETHX ACCEPTS A STRING OF HEX DIGITS FROM THE INPUT 
;              STREAM AND RETURNS THEIR VALUE AS A 16 BIT BINARY
;              INTEGER. IF MORE THAN 4 HEX DIGITS ARE ENTERED,
;              ONLY THE LAST 4 ARE USED. THE ROUTINE TERMINATES
;              WHEN A VALID DELIMITER IS ENCOUNTERED.
;              THE DELIMITER IS ALSO RETURNED AS OUTPUT OF THE ROUTINE
;              AT REGISTER D. ILLEGAL CHARACTERS (NO HEX DIGITS OR
;              DELIMITERS) CAUSE AN ERROR INDICATION. IF THE FIRST
;              (VALID) CHARACTER ENCOUNTERED IN THE INPUT STREAM IS
;              NOT A DELIMITER, GETHX WILL RETURN WITH THE CARRY BIT
;              SET TO 1, OTHERWISE THE CARRY BIT IS SET TO 0 AND THE
;              CONTENTS OF BC ARE UNDEFINED.

GETHX:
    PUSH H           ; Save registers HL.
    LXI H,00         ; Use registers HL to hold the digits. Initialize
                     ; the result to zero.
    MVI E,00         ; Initialize digit flag to false. Digit flag = 00
                     ; (false). No valid digits are received from console.
GHX05:
    CALL GETCH       ; Get a character. The character is in register C.
    CALL ECHO        ; Echo the character to console.
    CALL VALDL       ; See if it is a valid delimiter.
    JNC GHX10
    MOV D,C          ; Yes - All done, but want to return delimiter.
    PUSH H
    POP B            ; Move result to BC.
    POP H            ; Restore registers HL.
    MOV A,E          ; Get digit flag.
    ORA A            ; Set F/F.
    JNZ SRET         ; If flag not - 0, a number has been found.
    JZ FRET          ; Else, delimiter was the first character.
GHX10:
    CALL VALDG       ; If not delimiter, see if it is a valid hex digit.
    JNC ERROR        ; Error if not a valid digit.
    MOV A,C
    CALL HEX_BIN     ; Convert digit to binary value.
    MVI E,0FFH       ; Set digit flag to true.
    DAD H            ; *2.
    DAD H            ; *4.
    DAD H            ; *8.
    DAD H            ; *16. With these operations the partial result is
                     ; shifted 4 places to the left.
    MVI B,00         ; Clear upper 8 bits of BC pair.
    MOV C,A          ; Binary value of character into C.
    DAD B            ; Add this value to partial result.
    JMP GHX05        ; Get next character.
    
;*************************************************************************** 

; FUNCTION   : GETNM
; INPUTS     : C - COUNT OF NUMBERS TO FIND IN INPUT STREAM
; OUTPUTS    : TOP OF STACK - NUMBERS FOUND IN REVERSE ORDER
;              (LAST ON TOP OF STACK)
; CALLS      : GETHX,HILO,ERROR 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: GETNM FINDS A SPECIFIED COUNT OF NUMBERS, BETWEEN 
;              1 AND 3, INCLUSIVE, IN THE INPUT STREAM AND RETURNS
;              THEIR VALUES ON THE STACK AS A 16 BIT VALUE. IF
;              2 OR MORE NUMBERS ARE REQUESTED, THEN THE FIRST
;              MUST BE LESS THAN OR EQUAL TO THE SECOND, OR THE
;              FIRST AND SECOND NUMBERS WILL BE SET EQUAL. THE
;              LAST NUMBER REQUESTED MUST BE TERMINATED BY A
;              CARRIAGE RETURN OR AN ERROR INDICATION WILL RESULT.

GETNM:
    MVI L,3          ; Put maximum argument count into L.
    MOV A,C          ; Get the actual argument count.
    ANI 3            ; Force to maximum of 3.
    RZ               ; If 0, don't bother to do anything.
    MOV H,A          ; Else, put actual count into H.
GNM05:
    CALL GETHX       ; Get a number from input stream.
    JNC ERROR        ; Error if not three - Too few numbers received.
    PUSH B           ; Else, save number on stack.
    DCR L            ; Decrement maximum argument count.
    DCR H            ; Decrement actual argument count.
    JZ GNM10         ; Branch if no more numbers wanted.
    MOV A,D          ; Else, get number terminator to register A.
    CPI CR           ; See if it is a carriage return.
    JZ ERROR         ; Error if so - Too few numbers.
    JMP GNM05        ; Else, process next number.
GNM10:
    MOV A,D          ; When count 0, check last terminator.
    CPI CR           ; See if it is a carriage return.
    JNZ ERROR        ; Error if not carriage return.
    LXI B,0FFFFH     ; BC gets largest number.
    MOV A,L          ; Get what's left of maximum arg count.
    ORA A            ; Check for 0.
    JZ GNM20         ; If yes, 3 numbers were input.
GNM15:
    PUSH B           ; If not, fill remaining arguments with OFFFFH.
    DCR L
    JNZ GNM15
GNM20:
    POP B            ; Get the 3 arguments out of stack. Third (last)
                     ; number received.
    POP D            ; Second number received.
    POP H            ; First number received.
    CALL HILO        ; See if first >= second.
    JC GNM25         ; No - Branch.
    MOV D,H          ; Yes - Make second equal to the first.
    MOV E,L
GNM25:
    XTHL             ; Put first number on stack - Get return address
                     ; in register HL.
    PUSH D           ; Put second number on stack.
    PUSH B           ; Put third number on stack.
    PUSH H           ; Put return address on stack.
GNM30:
    DCR A            ; Decrement residual count.
    RM               ; If negative, proper results on stack.
    POP H            ; Else, get return address.
    XTHL             ; Replase result with return address. Skip one
                     ; number.
    JMP GNM30        ; Try again.

;***************************************************************************

; FUNCTION   : HILO
; INPUTS     : DE - 16 BIT INTEGER (SUBTRAHED)
;              HL - 16 BIT INTEGER (MINUEND)
; OUTPUTS    : IF HL < DE ZERO = 0 , CARRY = 1
;              IF HL > DE ZERO = 0 , CARRY = 0
;              IF HL = DE ZERO = 1 , CARRY = 0
; CALLS      : NOTHING 
; DESTROYS   : F/F
; DESCRIPTION: HILO COMPARES THE TWO 16 BIT INTEGERS IN HL AND DE.
;              THE INTEGERS ARE TREATED AS UNSIGNED NUMBERS. THE
;              CARRY BIT IS SET ACCORDING TO THE RESULT OF THE
;               COMPARISON.

HILO:
    PUSH B           ; Save registers BC.
    MOV B,A          ; Save register A in register B.
    MOV A,D          ; Get low byte of DE.
    XRA H            ; Find if the signs of operants are different.
    JM DIFF          ; Jump if different.
    
UNSIGN:
    MOV A,L          ; Overflow not possible - Perform unsigned comparison.
    SUB E            ; Compare low bytes.
    JZ EQUAL
    
    MOV A,H          ; Low bytes not equal - Compare high bytes.
    SBB D
    JC CYSET         ; Exit with carry = 1, zero = 0.
    JNC CYCLEAR      ; Exit with carry = 0, zero = 0.
    
EQUAL:
    MOV A,H          ; Low bytes are equal - Compare high bytes.
    SBB D            ; All flags are set by SBB.
    MOV A,B          ; Restore original contents of A.
    POP B            ; Restore original contents of BC.
    RET
    
DIFF:
    MOV A,L          ; Sign of two operands are different.
                     ; Overflow not possible - Perform unsigned comparison. 
    SUB E            ; Compare low bytes.
    MOV A,H          ; Compare high bytes.
    SBB D
    MOV A,H          ; Load high byte of minuend to obtain proper sign.
    JNC CYCLEAR      ; Exit with carry = 0, zero = 0.
    
CYSET:               ; Exit with carry = 1, zero = 0.
    ORI 1            ; Set zero = 0, sign = bit 7.
    STC              ; Carry = 1.
    MOV A,B          ; Restore original contents of A.
    POP B            ; Restore original contents of BC.
    RET

CYCLEAR:
    ORI 1            ; Set carry = 0, zero = 0, sign = bit 7.
    MOV A,B          ; Restore original contents of A.
    POP B            ; Restore original contents of BC.
    RET              ; Return with flags set as required.

;***************************************************************************

; FUNCTION   : LDELAY - LONG DELAY ROUTINE 
; INPUTS     : B NUMBER OF TIMES TO CALL DELAY 
; OUTPUTS    : NONE 
; CALLS      : DELAY 
; DESTROYS   : A,B,D,E,F/F
; DESCRIPTION: LONG DELAY ROUTINE USED FOR 2 sec DELAYS.

LDELAY:
    DCR B
    RZ
    LXI D,0FFFFH
    CALL DELAY ; Delay for aproximately 700 msec.
    JMP LDELAY
    
;***************************************************************************

; FUNCTION   : NMOUT
; INPUTS     : A - 8 BIT INTEGER 
; OUTPUTS    : NONE 
; CALLS      : ECHO, PRVAL 
; DESTROYS   : A,B,C,F/F
; DESCRIPTION: NNMOUT CONVERTS THE 8 BIT UNSIGNED INTEGER, IN THE REGISTER A 
;              INTO 2 HEX ASCII CHARACTERS. THE ASCII CHARACTERS ARE
;              THE ONES REPRESENTING THE 8 BITS.
;              THESE TWO CHARACTERS ARE SENT TO THE CONSOLE AT
;              THE CURRENT POSITION OF THE CONSOLE.

NMOUT:
    PUSH H           ; Save HL because it is destroyed by PRVAL.
    PUSH PSW         ; Save argument.
    RRC              ; Shift 1 bit position to right.
    RRC
    RRC
    RRC              ; Get upper 4 bits of the argument to low
                     ; 4 bit positions.
    ANI HCHAR        ; Mask out upper 4 bits - Want 1 hex char.
    MOV C,A          ; Move 4 bit to C for PRVAL.
    CALL PRVAL       ; Convert lower 4 bits to ASCII.
    CALL ECHO        ; Send to terminal.
    POP PSW          ; Restore argument.
    ANI HCHAR        ; Mask out 4 bits - Want 1 hex char.
    MOV C,A
    CALL PRVAL       ; Convert lower 4 bits to ASCII.
    CALL ECHO        ; Send to terminal.
    POP H            ; Restore saved value of HL.
    RET
    
;***************************************************************************

; FUNCTION   : PRVAL
; INPUTS     : C - INTEGER,RANGE 0 TO F 
; OUTPUTS    : C - ASCII CHARACTER 
; CALLS      : NOTHING 
; DESTROYS   : B,C,H,L,F/F
; DESCRIPTION: PRVAL CONVERTS A NUMBER IN THE RANGE 0 TO F HEX TO 
;              THE CORRESPONDING ASCII CHARACTER, 0-9, A-F. PRVAL
;              DOES NOT CHECK THE VALIDITY OF ITS INPUT ARGUMENT.

PRVAL:
    LXI H,DIGTB ; Address of table.
    MVI B,0 ; Clear high order bits of BC.
    DAD B ; Add digit value to HL address.
    MOV C,M ; Fetch character from memory.
    RET
    
;***************************************************************************

; FUNCTION   : REGDS 
; INPUTS     : NONE 
; OUTPUTS    : NONE 
; CALLS      : ECHO,NMOUT,CROUT 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: REGDS DISPLAYS THE CONTENTS OF THE REGISTER SAVE 
;              LOCATIONS, IN FORMATED FORM, ON THE CONSOLE. THE
;              DISPLAY IS DRIVEN FROM TABLE, RTAB, WHICH
;              CONTAINS THE REGISTER'S PRINT SYMBOL, SAVE
;              LOCATION ADDRESS, AND LENGTH (8 OR 16 BITS).

REGDS:
    LXI H,RTAB       ; Load HL with address of start of table.
REG05:
    MOV C,M          ; Get print symbol of register.
    MOV A,C
    ORA A            ; Test for 0 - End of table.
    JNZ REG10        ; If not end, branch.
    CALL CROUT       ; Else, carriage return/line feed to end.
    RET
REG10:
    CALL ECHO        ; Echo character to console.
    MVI C,'='
    CALL ECHO        ; Output equal sign, for example A=.
    INX H            ; Point to start of save location address.
    MOV E,M          ; Get low byte of save location address to E.
    MVI D,(MNSTK >> 8) ; Put high byte of save location address into D.
    INX H            ; Point to length flag.
    LDAX D           ; Get contents of save address to register A.
    CALL NMOUT       ; Display to console.
    MOV A,M          ; Get length flag.
    ORA A            ; Set sign F/F.
    JZ REG15         ; If 0, register is 8 bits long.
    DCX D            ; Else, register is 16 bit long - Display more.
    LDAX D           ; Get lower 8 bits.
    CALL NMOUT       ; Display them to console.
REG15:
    MVI C,' '        ; A space is used as a separator between registers.
    CALL ECHO
    INX H            ; Point to start of next table entry.
    JMP REG05        ; Display next register.

;***************************************************************************

; FUNCTION   : RGADR
; INPUTS     : C - CHARACTER DENOTING REGISTER
; OUTPUTS    : BC - ADDRESS OF ENTRY IN RTAB CORRESPONDING TO REGISTER 
; CALLS      : ERROR 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: RGADR TAKES A SINGLE CHARACTER AS INPUT. THIS
;              CHARACTER DENOTES A REGISTER. RGADR SEARCHES THE
;              TABLE RTAB FOR A MATCH ON THE INPUT ARGUMENT. IF
;              ONE OCCURS, RGADR RETURNS THE ADDRESS OF THE
;              ADDRESS OF THE SAVE LOCATION CORRESPONDING TO THE
;              REGISTER. THIS ADDRESS POINT INTO RTAB. IF NO
;              MATCH OCCURS, THEN THE REGISTER IDENTIFIER IS
;              ILLEGAL AND CONTROL IS PASSED TO THE ERROR
;              ROUTINE.

RGADR:
    LXI H,RTAB       ; HL gets address of table start.
    LXI D,RTABS      ; DE gets size of a table entry.
RGA05:
    MOV A,M          ; Get register identifier.
    ORA A            ; Check for table end (identifier is 0).
    JZ ERROR         ; If at end of table, argument is illegal.
    CMP C            ; Else, compare table entry and argument.
    JZ RGA10         ; If equal, we have fount what we are looking for.
    DAD D            ; Else, increment table pointer to next entry.
    JMP RGA05        ; Try again.
RGA10:
    INX H            ; If a match, increment table pointer to
    MOV B,H          ; save location address.
    MOV C,L          ; Return this value in registers BC.
    RET

;***************************************************************************

; FUNCTION: SRET 
; INPUTS : NONE 
; OUTPUTS : CARRY = 1 
; CALLS : NOTHING 
; DESTROYS: CARRY
; DESCRIPTION: SRET IS JUMPED TO BY ROUTINES WISHING TO RETURN 
;              SUCCESS. SRET SETS THE CARRY TRUE AND THEN RETURNS
;              TO THE CALLER OF THE ROUTINE INVOKING SRET.

SRET:
    STC              ; Set carry true.
    RET              ; Return appropriately.
    
;*************************************************************************** 

; FUNCTION   : STHFO
; INPUTS     : DE - 16 BIT ADDRESS OF BYTE TO BE STORED INTO 
; OUTPUTS    : NONE 
; CALLS      : STHLF 
; DESTROYS   : A,B,C,H,L,F/F
; DESCRIPTION: STHFO CHECKS THE HALF BYTE FLAG IN TEMP TO SEE IF 
;              IT IS SET TO LOWER. IF SO, STHFO STORES A 0 TO FILL
;              OUT THE LOWER HALF OF THE ADDRESSED BYTE.
;              OTHERWISE, THE ROUTINE TAKES NO ACTION.

STHFO:
    LDA TEMP         ; Get half byte flag.
    ORA A            ; Set F/F.
    RNZ              ; If set to upper, don't do anything.
    MVI C,0          ; Else, want to store the value 0 to fill out the
                     ; Lower half.
    CALL STHLF       ; Do it.
    RET
    
;*************************************************************************** 

; FUNCTION: STHLF
; INPUTS     : C - 4 BIT VALUE TO BE STORED 
;              DE - 16 BIT ADDRESS
; OUTPUTS    : NONE 
; CALLS      : NOTHING 
; DESTROYS   : A,B,C,H,L,F/F
; DESCRIPTION: STHLF TAKES THE 4 BIT VALUE IN C AND STORES IT IN
;              HALF OF THE BYTE ADDRESSED BY REGISTERS DE. THE
;              HALF BYTE USED (EITHER UPPER OR LOWER) IS DENOTED
;              BY THE VALUE OF THE FLAG IN TEMP. STHLF ASSUMES
;              THAT THIS FLAG HAS BEEN PREVIOUSLY SET (NORMALY
;              BY ENTER_CMD).

STHLF:
    PUSH D
    POP H            ; Move address of byte into HL.
    MOV A,C          ; Get value.
    ANI 0FH         ; Force to 4 bit length.
    MOV C,A          ; Put value back.
    LDA TEMP         ; Get half byte flag.
    ORA A            ; Check for lower half.
    JNZ STH05        ; Branch if not.
    MOV A,M          ; Else, get byte and use lower half.
    ANI 0F0H         ; Clear lower 4 bits.
    ORA C            ; Or in value.
    MOV M,A          ; Put byte back.
    RET
STH05:
    MOV A,M          ; Use upper half, get byte.
    ANI 0FH          ; Clear upper 4 bits.
    MOV B,A          ; Save byte in B.
    MOV A,C          ; Get value.
    RRC
    RRC
    RRC
    RRC              ; Align to upper 4 bits.
    ORA B            ; OR in original lower 4 bits.
    MOV M,A          ; Put new byte back.
    RET
    
;***************************************************************************

; FUNCTION   : VALDG 
; INPUTS     : C - ASCII CHARACTER
; OUTPUTS    : CARRY - 1 IF CHARACTER REPRESENTS VALID HEX DIGIT 
;                    - 0 OTHERWISE
; CALLS      : FRET, SRET 
; DESTROYS   : A,F/F
; DESCRIPTION: VALDG RETURNS SUCCESSFUL IF ITS INPUT ARGUMENT IS 
;              AN ASCII CHARACTER REPRESENTING A VALID HEX DIGIT
;              (0-9,A-F). OTHERWISE IT RETURNS WITH FAILURE.

VALDG:
    MOV A,C
    CPI '0'          ; Test character against '0'.
    JM FRET          ; If ASCII code is less, cannot be valid digit.
    CPI '9'          ; Else, see if it is in the range '0-9'.
    JM SRET          ; Code between 'O' and .91.
    JZ SRET          ; Code equal '9'.
    CPI 'A'          ; Not a digit - Try for a hex letter.
    JM FRET          ; No - code between '9' and 'A'.
    CPI 'G'
    JP FRET          ; No - Code greater than 'F..
    JMP SRET         ; Code is 'A' to ?F1, inclusive.
    
;******************************************************************************
;
; FUNCTION   : VALDL 
; INPUTS     : C - CHARACTER
; OUTPUT     : CARRY - 1 IF INPUT ARGUMENT IS A VALID DELIMETER 
;                    - 0 OTHERWISE
; CALLS      : SRET, FRET
; DESCRIPTION: VALDL RETURNS SUCCESSFUL IF ITS INPUT ARGUMENT IS A 
;              VALID DELIMITER CHARACTER (SPACE, COMMA, CARRIAGE RETURN).
;              OTHERWISE IT RETURNS WITH FAILURE.

VALDL:
    MOV A,C ; Move character to register A.
    CPI ',' ; Check for comma.
    JZ SRET
    CPI CR ; Check for carriage return.
    JZ SRET
    CPI ' ' ; Check for space.
    JZ SRET
    JMP FRET ; Error if none of the above.
    
;***************************************************************************
;
; CRT MONITOR MONITOR TABLES
;
;***************************************************************************

CONAD:   DB LETRC,LETRR,LETRT,BLANK    ; When you are in CRT
CONDAT:  DB BLANK,BLANK                ; monitor display 'Crt'.

SIGN_ON: DB CR,LF,LF," MICROKIT MONITOR Ver. 5.12 ",CR,LF,00

CADR:
    DW 0             ; Dummy character.
    DW HELP_CMD      ; Help command.
    DW FILL_CMD      ; Fill memory block command.
    DW XREG_CMD      ; Examine Registers command.
    DW TRACE_CMD     ; Trace command - Execute one instruction.
    DW SUBST_CMD     ; Substitute memory command.
    DW MOVE_CMD      ; Move memory block command.
    DW ENTER_CMD     ; Enter hex digits command.
    DW GO_CMD        ; Execute user program command.
    DW DISPL_CMD     ; Display memory command.
    DW OUT_CMD       ; Output data to port command.
    DW IN_CMD        ; Input data from port command.
    DW PORT_CMD      ; Select active serial port for download command.
    DW WRITE_CMD     ; Write data to tape.
    DW READ_CMD      ; Read from tape command.
    DW LOAD_CMD      ; Download command.

CTAB:      ; Table of valid command characters.
    DB 'L' ; Download
    DB 'R' ; Read from tape
    DB 'W' ; Write to tape
    DB 'P' ; Select active download Port
    DB 'I' ; Input data from port.
    DB 'O' ; Output data to port.
    DB 'D' ; Display
    DB 'G' ; Go
    DB 'E' ; Enter
    DB 'M' ; Move
    DB 'S' ; Substitute memory
    DB 'T' ; Trace
    DB 'X' ; examine Registers
    DB 'F' ; Fill
    DB 'H' ; Help

NCMDS EQU ($-CTAB)           ; Number of valid commands.

DIGTB: DB "0123456789ABCDEF" ; Table to convert hex digits to ASCII
                             ; characters.

RTAB:                  ; Table of register information.
    DB 'A'             ; Register identifier.
    DB (ASAV & 0FFH)   ; Address of register save location.
    DB 0               ; Length flag - 0=8 bits, 1=16 bits.
RTABS EQU ($-RTAB)     ; Size of an entry in this table.
    DB 'B'
    DB (BSAV & 0FFH)
    DB 0
    DB 'C'
    DB (CSAV & 0FFH)
    DB 0
    DB 'D'
    DB (DSAV & 0FFH)
    DB 0
    DB 'E'
    DB (ESAV & 0FFH)
    DB 0
    DB 'F'
    DB (FSAV & 0FFH)
    DB 0
    DB 'I'
    DB (ISAV & 0FFH)
    DB 0
    DB 'H'
    DB (HSAV & 0FFH)
    DB 0
    DB 'L'
    DB (LSAV & 0FFH)
    DB 0
    DB 'M'
    DB (HSAV & 0FFH)
    DB 1
    DB 'S'
    DB ((SSAV+1) & 0FFH)
    DB 1
    DB 'P'
    DB ((PSAV+1) & 0FFH)
    DB 1
    DB 0               ; End of table markers.
    DB 0
    
;*****************************************************************************

HELP_SCREEN_1 DB CR,LF

s11  DB "        M I K R O K I T    M O N I T O R    C O M M A N D S           Page 1",0DH,0AH,0AH

S12  DB "   Examine/Modify CPU Registers :",CR,LF
S13  DB "   X [Register Name] [Data] ... [,Data] <CR>",CR,LF,LF

S14  DB "   Examine/Modify Memory :",CR,LF
S15  DB "   S <Start Address> <,> [Data] ... [,] [Data] <CR>",CR,LF,LF

S16  DB "   Display Memory :",CR,LF
S17  DB "   D [Start Address] <,> [End Address] <CR>",CR,LF,LF

S18  DB "   Move Data :",CR,LF
S19  DB "   M <Start Address> <,> <End Address> <,> <Destination Address> <CR>",CR,LF,LF

S110 DB "   Execute Program :",CR,LF
S111 DB "   G [Start Address] <CR>",CR,LF,LF

S112 DB "   Execute One Program Instruction :",CR,LF
S113 DB "   T [Start Address] <,> [Address] ... <,> [Address] <CR>",CR,LF,LF

S114 DB " Items inside <> are required. Items inside [] are optional.",CR,LF,LF

S115 DB "Press any key for next page / Esc to exit",CR,LF,00 



HELP_SCREEN_2 DB CR,LF

S21  DB "        M I K R O K I T    M O N I T O R    C O M M A N D S          Page 2",0DH,0AH,0AH

S22  DB "   Fill Memory with Data :",CR,LF
S23  DB "   F <Start Address> <,> <End Address> <,> <Data> <CR>",CR,LF,LF 
S24  DB "   Enter Bytes in Memory :",CR,LF

S25  DB "   E <Start Address> <CR>     <Data><Data>...<Data><ESC>",CR,LF,LF

S26  DB "   Input Data from a Port :",CR,LF

S27  DB "   I <Port Address> <,> [,] ... [,] <CR>",CR,LF,LF
S28  DB "   Output Data to a Port :",CR,LF
S29  DB "   O <Port Address> <,> <Data> <CR>",CR,LF,LF

S210 DB "   Select the active serial Port for the Load Command :  P",CR,LF,LF

S211 DB "   Load an INTEL HEX File  :  L",CR,LF
S212 DB "   Write Data to Tape      :  W",CR,LF
S213 DB "   Read Data from Tape     :  R",CR,LF
S214 DB "   Display Help screens    :  H",CR,LF,LF

S215 DB " Items inside <> are required. Items inside [] are optional.",CR,LF,LF



;***************************************************************************
;
;                   CASSETE RECORDER INTERFACE PROGRAMS
;
;*************************************************************************** 

; THESE programs are utility programs for writing and reading a
; FILE TO OR FROM A CASSETTE RECORDER. THE PROGRAMS CAN BE CALLED 
; FROM KEYBOARD MONITOR OR FROM CRT MONITOR.

;***************************************************************************
;
; WRITE PROGRAM AND SUBROUTINES
;
;***************************************************************************

; FUNCTION   : KWRITE - KEYBOARD MONITOR WRITE MAIN ROUTINE 
; INPUTS     : NONE 
; OUTPUTS    : NONE
; CALLS      : CLEAR,ERR,HILO,RDKBD,WRITE,DI SADR 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: THIS IS THE MAIN ROUTINE FOR WRITING ON TAPE WHEN
;              KEYBOARD MONITOR IS USED. ASKS FOR THE START AND
;              END ADDRESSES. ASKS USER TO START TAPE RECORDER,
;              WRITES THE DATA BLOCK ON TAPE, DISPLAYS THE START
;              AND END ADDRESSES AND GIVES CONTROL TO KEYBOARD
;              MONITOR PROGRAM.

    ORG 1000H        ; Start address for write program.
    
KWRITE:
    MVI A,UNMSK
    SIM              ; Unmask interrupts from the keyboard.
    MVI B,0
    CALL CLEAR       ; Clear the display.
    
    LXI H,KMES1      ; Display message " S_Ad
    MVI A,ADFLD      ; Use address field of display.
    MVI B,NODOT      ; No dot on display.
    CALL OUTPT       ; Display the message.
    MVI B,ADFLD      ; Use address field to get start address.
    CALL GTHEX       ; Get 4 hex digits.
    XCHG             ; Save input in H,L
    JNC ERR          ; No digits entered exit to error routine.
    SHLD STADR       ; Store start address in RAM.
    
    LXI H,KMES2      ; Display message " E_Ad ".
    MVI A,ADFLD      ; Use address field of display.
    MVI B,NODOT      ; No dot on display.
    CALL OUTPT       ; Display the message.
    MVI B,ADFLD      ; Use address field to get end address.
    CALL GTHEX       ; Get 4 hex digits.
    XCHG             ; Save input in H,L.
    JNC ERR          ; If no digits entered exit to error routine.
    SHLD EDADR       ; Store end address in RAM.
    
    XCHG             ; Put end address in D,E.
    LHLD STADR       ; Put start address in H,L.
    CALL HILO        ; Find if start address > end address.
    JNC ERR          ; if not exit to error routine.
    
    LXI H,KMES3      ; Display message " S_rC " (Start recorder).
    MVI A,ADFLD      ; Use address field of display.
    MVI B,NODOT      ; No dot on display.
    CALL OUTPT       ; Display message.
    MVI A,DTFLD      ; Use data field of display.
    MVI B,NODOT      ; No dot.
    CALL OUTPT       ; Display at data field of display.
    
    CALL RDKBD       ; Get user input .
    CPI PERIOD       ; User pressed or 11 " ?
    JZ NOERRO
    CPI COMMA
    JNZ ERR          ; No - Error.
NOERRO:
    LXI H,KMES4      ; Display message " rECOrd ".
    MVI A,ADFLD      ; Use address field of display.
    MVI B,NODOT      ; No dot on display.
    CALL OUTPT       ; Display message.
    MVI A,DTFLD      ; Use data field of display.
    MVI B,NODOT      ; No dot.
    CALL OUTPT       ; Display at data field of display.
    
    CALL WRITE       ; Write data block on tape.
    MVI B,0
    CALL CLEAR       ; Clear the display.
    CALL DISADR     ; Display start and end addresses.
    JMP CMMND        ; Exit to keyboard monitor.

;***************************************************************************

; FUNCTION   : BURST - WRITE 1/3 OF BIT ON TAPE 
; INPUTS     : IF SOE (BIT 6) IS 'O' NO BURST (SPACE).
;              IF SOE IS '1' OUTPUT A BURST AT SOD LINE.
;              (SOE=SOD OUTPUT ENABLE, SOD=SERIAL OUTPUT DATA)
; OUTPUTS    : NONE 
; CALLS      : NOTHING 
; DESTROYS   : A,D,E,F/F
; DESCRIPTION: OUTPUTS A TONE OR NOTHING TO THE RECORDER FOR 1/3 
;              OF BIT TIME.

BURST:
    MVI D,CYCNO      ; Set number of cycles per burst.
BU1:
    SIM              ; If SOD enable is set output data bit.
    MVI E,HAFCYC
BU2:
    DCR E
    JNZ BU2
    XRI 80H          ; Complement SOD data bit.
    DCR D            ; Continue until burst or pause finished.
    JNZ BU1
    RET

;***************************************************************************    

; FUNCTION   : WR_BYTE - WRITE ONE BYTE ON TAPE 
; INPUTS     : C THE BYTE TO BE WRITTEN ON TAPE 
; OUTPUTS    : NONE 
; CALLS      : BURST 
; DESTROYS   : A.B.C.D.E
; DESCRIPTION: WRITES THE BYTE IN REGISTER C TO TAPE. THE BYTE
;              WRITTEN ON TAPE HAS 8 BITS DATA AND 1 STOP BIT
;              A "1" IS WRITTEN AS 2/3 BURST AND 1/3 SPACE.
;              A "0" IS WRITTEN AS 1/3 BURST AND 2/3 SPACE.

WR_BYTE:
    DI
    PUSH D           ; Save DE used by burst.
    MVI B,9          ; 8 data bits and 1 stop bit.
TO1:
    XRA A            ; Clear cary bit.
    MVI A,0C0H       ; Set code for tone burst.
    CALL BURST
    MOV A,C
    RAR              ; Hove next data bit in carry.
    MOV C,A
    MVI A,01H        ; Set bit to be repeatedly complemented
                     ; in burst routine.
    RAR
    RAR
    CALL BURST       ; Output a tone burst or a space.
    XRA A ; Clear A.
    CALL BURST       ; Output a space.
    DCR B
    JNZ TO1          ; Repeat until byte finished.
    POP D            ; Restore DE.
    EI
    RET
    
;***************************************************************************

; FUNCTION   : WR_4BYTE - WRITE THE 4 HEX DIGITS TO TAPE 
; INPUTS     : H,L 
; OUTPUTS    : NONE 
; CALLS      : WR_BYTE 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: WRITES THE 16 BIT INTEGER IN HL TO TAPE.

WR_4BYTE:
    MOV C,L
    CALL WR_BYTE     ; Write low byte.
    MOV C,H
    CALL WR_BYTE     ; Write high byte.
    RET

;***************************************************************************

; FUNCTION   : WR_LEAD - WRITE A LEADER ON TAPE 
; INPUTS     : NONE 
; OUTPUTS    : NONE 
; CALLS      : BURST 
; DESTROYS   : A,B,C,D,E,F/F
; DESCRIPTION: WRITES A LONG TONE BURST AS LEADER ON TAPE.
;              IT IS USED BY RECORDER ELECTRONICS AND ALLOWS
;              THEM TO STABILIZE AT THE RIGHT LEVEL.

WR_LEAD:
    MVI C,LEADER     ; Set up leader burst length.
    MVI A,0C0H       ; Set code for tone burst.
    MVI B,TIMES      ; Make the leader long.
BR1:
    CALL BURST       ; Output a tone burst.
    DCR C
    JNZ BR1
    DCR B
    JNZ BR1
    RET

;***************************************************************************

; FUNCTION   : WRITE - WRITE A FILE ON TAPE 
; INPUTS     : NONE 
; OUTPUTS    : NONE
; CALLS      : WRJ.EAD,BURST,WR_4BYTE,WR_BYTE,HILO 
; DESTROYS   : A,B,C,D,E,H,L,F/F 
; DESCRIPTION: WRITES ON TAPE:
;                   1. THE LEADER
;                   2. START AND END ADDRESSES
;                   3. THE DATA BLOCK FROM MEMORY
;                   4. ONE BYTE CHECKSUM.


WRITE:
    CALL WR_LEAD     ; Write leader.
    XRA A
    CALL BURST       ; Write a 1/3 of bit time space so that
                     ; start of the first data byte can be detected.
    LHLD STADR
    CALL WR_4BYTE    ; Write start address on tape.
    LHLD EDADR
    CALL WR_4BYTE    ; Write end address on tape.
    XCHG 
    INX D
    LHLD STADR
    XRA A
    STA CHKSUM
NEXBYT:
    MOV C,M          ; Get a data byte from memory.
    LDA CHKSUM
    ADD C            ; Add it to checksum.
    STA CHKSUM
    CALL WR_BYTE     ; Write the data byte on tape.
    INX H
    CALL HILO        ; Is it the last byte ?
    JC NEXBYT        ; If not go get next.
    LDA CHKSUM
    MOV C,A
    CALL WR_BYTE     ; Write the checksum on tape.
    RET

;***************************************************************************
;
;                  TAPE READ PROGRAM AND SUBROUTINES
;
;***************************************************************************

; FUNCTION   : KREAD - TAPE READ MAIN ROUTINE FOR KEYBOARD MONITOR 
; INPUTS     : NONE 
; OUTPUTS    : NONE
; CALLS      : RDKBD,ERR,READ,DI SADR 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: ASKS THE USER TO START TAPE RECORDER (PLAYBACK).
;              READS START, END ADDRESSES AND THE DATA BYTES.
;              ADDS ALL DATA BYTES AND COMPARES THE RESULT WITH
;              THE CHECKSUM WRITTEN ON TAPE, IF THE TWO CHECKSUMS
;              ARE NOT THE SAME EXITS TO ERROR ROUTINE. ELSE
;              DISPLAYS START AND END ADDRESSES AND GIVES CONTROL
;              TO KEYBOARD MONITOR.


    ORG 1300H        ; Start address for the tape read program.
    
KREAD:
    MVI A,UNMSK
    SIM              ; Unmask interrupts from the keyboard.
    LXI H,KMES3      ; Display message " S_rC " (start tape recorder).
    MVI A,ADFLD      ; Use address field of display.
    MVI B,NODOT      ; No dot on display.
    CALL OUTPT       ; Display message.
    MVI A,DTFLD      ; Use data field of display.
    MVI B,NODOT      ; No dot.
    CALL OUTPT       ; Display at data field of display.
    
    CALL RDKBD       ; Wait for the user's answer.
    CPI PERIOD
    JZ NOERR9
    CPI COMMA
    JNZ ERR          ; If not a valid delimiter exit to error routine.
NOERR9:
    LXI H,KMES5      ; Display message " rEAd ".
    MVI A,ADFLD      ; Use address field of display.
    MVI B,NODOT      ; No dot on display.
    CALL OUTPT       ; Display message.
    MVI A,DTFLD      ; Use data field of display.
    MVI B,NODOT      ; No dot.
    CALL OUTPT       ; Display at data field of display.
    
    CALL READ        ; Read the file from tape.
    JNC ERR          ; If checksum is not o.k exit to error.
    CALL DISADR      ; Else display start and end addresses.
    JMP CMMND        ; Give control to keyboard monitor.

;***************************************************************************

; FUNCTION   : DISADR - DISPLAY START AND END ADDRESSES ON MIKROKIT DISPLAY 
; INPUTS     : NONE 
; OUTPUTS    : NONE 
; CALLS      : OUTPT,LDELAY,HXDSP 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: DISPLAYS ON MIKROKIT DISPLAY THE START AND END ADDRESSES 

DISADR:
    LXI H,KMES1
    MVI A,ADFLD      ; Use address field of display.
    MVI B,NODOT      ; No dot on display.
    CALL OUTPT       ; Display the message " S_Ad ".
    MVI B,DELY1
    CALL LDELAY      ; Wait for 2 sec.
    LHLD STADR
    XCHG
    CALL HXDSP
    MVI A,ADFLD      ; Use address field of display.
    MVI B,NODOT      ; No dot on display.
    CALL OUTPT       ; Display the start address.
    MVI B,DELY2
    CALL LDELAY      ; Wait for 2 sec.
    LXI H,KMES2
    MVI A,ADFLD      ; Use address field of display.
    MVI B,NODOT      ; No dot on display.
    CALL OUTPT       ; Display message " E_Ad 11.
    MVI B,DELY1
    CALL LDELAY      ; Wait for 2 sec.
    LHLD EDADR
    XCHG
    CALL HXDSP
    MVI A,ADFLD      ; Use address field of display.
    MVI B,NODOT      ; No dot on display.
    CALL OUTPT       ; Display the end address.
    MVI B,DELY2
    CALL LDELAY      ; Wait for 2 sec.
    MVI B,0
    CALL CLEAR       ; Clear the display.
    RET
    
;***************************************************************************    

; FUNCTION   : D_STR_END_ADDR 
; INPUTS     : NONE 
; OUTPUTS    : NONE 
; CALLS      : DISPL_MES,NMOUT 
; DESTROYS   : A,B,C,H,L,F/F
; DESCRIPTION: DISPLAYS THE START AND THE END ADDRESSES ON CRT. 

D_STR_END_ADDR:
    LXI H,MES1
    CALL DISPL_MES ; Display message "START ADDRESS :"
    LHLD STADR 
    MOV A,H
    CALL NMOUT ; Display the high 2 hex digits.
    MOV A,L
    CALL NMOUT ; Display the low 2 digits.
    
    CALL CROUT
    
    LXI H,MES3
    CALL DISPL_MES ; Display message "END ADDRESS :".
    LHLD EDADR 
    MOV A,H
    CALL NMOUT ; Display the high 2 hex digits.
    MOV A,L
    CALL NMOUT ; Display the low 2 digits.
    
    RET

;***************************************************************************

; FUNCTION   : BITIN - GET A SAMPLE FROM TAPE 
; INPUTS     : NONE
; OUTPUTS    : CY THE SAMPLE FROM SID LINE 
; CALLS      : NONE 
; DESTROYS   : A,E,F/F
; DESCRIPTION: READS A PART OF A DATA BIT FROM TAPE BY TAKING A 
;              SAMPLE FROM SID LINE.

BITIN:
    MVI E,CKRATE
BI1:
    DCR E
    JNZ BI1          ; Limit input sampling rate.
    RIM              ; Sample SID line.
    RAL              ; Move data in carry bit.
    RET

;***************************************************************************

; FUNCTION   : TAPE IN - READ A BYTE FROM TAPE 
; INPUTS     : NONE
; OUTPUTS    : C THE BYTE FROM TAPE 
; CALLS      : BITIN 
; DESTROYS   : A,B,CfD,E,F/F
; DESCRIPTION: READS 8 DATA 8ITS AND 1 STOP BIT FROM TAPE BY 
;              TAKING SUCCESSIVE SAMPLES FROM SID LINE.

TAPEIN:
    MVI B,08H        ; Read 8 data bits.
TI1:
    MVI D,0          ; Clear up/down counter.
TI2:
    DCR D            ; Decrement counter each time 1 is read.
    CALL BITIN       ; Read a sample.
    JC TI2           ; Repeat if still at 1 level.
    CALL BITIN 
    JC TI2
TI3:
    INR D            ; Increment counter each time 0 is read.
    CALL BITIN
    JNC TI3          ; Repeat each time 0 is read.
    CALL BITIN 
    JNC TI3
    MOV A,D
    RAL              ; Move counter MSB into carry.
    MOV A,C
    RAR              ; Move data bit recieved into
    MOV C,A          ; Byte register C.
    DCR B
    JNZ TI1          ; Repeat until byte assembled.
TI4:
    CALL BITIN 
    JC TI4
    CALL BITIN 
    JC TI4
TI5:
    CALL BITIN 
    JNC TI5
    CALL BITIN 
    JNC TI5
    RET
    
;***************************************************************************    

; FUNCTION   : RDADR - READ ADDRESS FIELD FROM TAPE
; INPUTS     : H,L THE ADDRESS SAVE LOCATION
; OUTPUTS    : NONE
; CALLS      : TAPEIN
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: READS A 16 BIT INTEGER (4 HEX DIGITS) FROM 
;              TAPE INTO H,L REGISTER.

RDADR:
    CALL TAPEIN      ; Read low byte from tape.
    MOV M,C
    CALL TAPEIN      ; Read high byte from tape.
    INX H 
    MOV M,C
    RET
    
;***************************************************************************

; FUNCTION   : RLEAD - READ THE LEADER FROM TAPE
; INPUTS     : NONE
; OUTPUTS    : NONE
; CALLS      : BITIN
; DESTROYS   : A,B,C,E,F/F
; DESCRIPTION: READS THE LONG TONE BURST (LEADER) FROM TAPE 

RLEAD:
    MVI B,REPEAT     ; Take samples from leader.
PLAYBK:
    MVI C,LDRCHK     ; Number of succesive 1 to be read to
PB1:
    CALL BITIN       ; verify that the leader is present.
    JNC PLAYBK
    DCR C
    JNZ PB1
    DCR B
    JNZ PB1          ; Make sure that you reading the leader.
PS1:
    CALL BITIN       ; Now you are reading the leader.
    JC PS1
    CALL BITIN
    JC PS1
PS2:
    CALL BITIN
    JNC PS2
    CALL BITIN
    JNC PS2
    RET
    
;***************************************************************************

; FUNCTION   : READ - READ A FILE FROM TAPE
; INPUTS     : NONE
; OUTPUTS    : NONE
; CALLS      : RLEAD,RDADR,TAPEIN,HILO,SRET,FRET
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: READS THE LEADER THE START AND END ADDRESSES, THE
;              DATA BLOCK AND THE CHECKSUM.

READ:
    CALL RLEAD       ; Read the leader.
    LXI H,STADR
    CALL RDADR       ; Read start address.
    LXI H,EDADR
    CALL RDADR       ; Read end address.
    LHLD EDADR 
    INX H
    XCHG
    LHLD STADR
    XRA A
    STA CHKSUM
REDNEX:
    PUSH D
    CALL TAPEIN      ; Read 1 data byte.
    MOV M,C          ; Store it to RAM location pointed by H,L.
    LDA CHKSUM
    ADD C            ; Add data byte to checksum.
    STA CHKSUM
    INX H
    POP D
    CALL HILO        ; Is it the last data byte ?
    JC REDNEX        ; If not go get the next one.
    CALL TAPEIN      ; Read checksum byte.
    LDA CHKSUM
    CMP C
    JZ SRET          ; If checksum is o.k return true.
    JMP FRET         ; Else return false.
    
;***************************************************************************
;
;          TAPE PROGRAM MESSAGES TO MICROKIT'S DISPLAY
;
;***************************************************************************

KMES1: DB 05H,15H,0AH,0DH         ; "S_Ad"
KMES2: DB 0EH,15H,0AH,0DH         ; "E_Ad"
KMES3: DB 05H,15H,14H,0CH,15H,15H ; "S_rC_"
KMES4: DB 14H,0EH,0CH,00H,14H,0DH ; "rECOrd"
KMES5: DB 14H,0EH,0AH,0DH,15H,15H ; "rEAd_"

;***************************************************************************
;
;           TAPE PROGRAM MESSAGES TO CRT SCREEN
;
;*************************************************************************** 

MES1: DB CR,LF," START ADDRESS :",00
MES2: DB " NOT VALID ADDRESS",CR,LF,00
MES3: DB " END ADDRESS   :",00
MES4: DB CR,LF," START ADDRESS > END ADDRESS",00
MES5: DB CR,LF," START TAPE RECORDER"
      DB CR,LF," PRESS RETURN AFTER TAPE START ",00
MES6: DB CR,LF," WRITING ON TAPE. PLEASE WAIT ... ",00
MES7: DB CR,LF," END OF WRITING",CR,LF,00
MES8: DB CR,LF," READING FROM TAPE. PLEASE WAIT ...",00
MES9: DB CR,LF," READING ERROR",CR,LF,00

;****************************************************************************** 
;
; DOWNLOAD PROGRAM AND SUBROUTINES
;
;******************************************************************************
; THE CONNECTION BETWEEN THE IBM-PC COMPUTER AND "MICROKIT" IS MADE 
; THROUGHT RS232 FROM A SERIAL PORT OF THE IBM-PC TO ONE OF THE SERIAL 
; PORTS OF MICROKIT DB9-1 OR DB9-2.
; THE PROGRAM FINDS THE ACTIVE SERIAL PORT FROM THE STORED VALUE IN DPORT.
; TO SELECT THE ACTIVE PORT RUN THE DPORT ROUTINE.
; THE TRANSMISSION SPEED IS ALLWAYS 9600 BPS, 8 DATA BITS, NO PARITY, 2 STOP. 
;******************************************************************************

;FUNCTION    : KDNLD - DOWNLOAD MAIN ROUTINE FOR THE KEYBOARD MONITOR 
; INPUTS     : NONE 
; OUTPUTS    : NONE
; CALLS      : CLEAR, GETFL,OUTDS,LDELAY 
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: THIS IS THE MAIN ROUTINE FOR DOWNLOADING A .HEX 
;              FILE TO MICROKIT WHEN KEYBOARD MONITOR IS USED.
;              THE PROGRAM LOADS THE CONTENTS OF .HEX FILE IN
;              MICROKIT'S MEMORY AND GIVES CONTROL TO KEYBOARD
;              MONITOR PROGRAM.


    ORG 1600H        ; The program starts at this location.

KDNLD:
    MVI B,NODOT      ; No dot in address field.
    CALL CLEAR       ; Clear display.
    
    LXI H,DMES       ; Address of 'START LOADING' message.
    CALL OUTDS       ; Display the message.
    
    CALL GETFL       ; Start downloading.
    INR A            ; Is there a transmission error code.
    JZ KDLD1         ; Yes - Display error message.
    
    MVI B,NODOT      ; No - No dot in address field.
    CALL CLEAR       ; Clear display.
    LXI H,ENDM       ; Address of message 'TRANSMISSION END' .
    CALL OUTDS       ; Display the message.
    
    MVI B,0AH
    CALL LDELAY      ; Wait for 2 seconds.
    MVI B,NODOT      ; No - No dot in address field.
    CALL CLEAR       ; Clear display.
    MVI A,UNMSK      ; Unmask interrupts from the keyboard.
    SIM
    JMP CMMND        ; Exit to keyboard monitor.
    
KDLD1:
    MVI B,NODOT      ; No dot in address field.
    CALL CLEAR       ; Clear display.
    LXI H,BADM       ; Address of transmission error message.
    CALL OUTDS       ; Display the message.
    MVI B,0AH
    CALL LDELAY      ; Wait for 2 seconds.
    MVI B,NODOT      ; No - No dot in address field.
    CALL CLEAR       ; Clear display.
    MVI A,UNMSK      ; Unmask interrupts from the keyboard.
    SIM
    JMP CMMND        ; Exit to keyboard monitor.
    
;***************************************************************************

; FUNCTION   : GETFL 
; INPUTS     : NONE
; OUTPUTS    : A-OOH NO TRANSMISSION ERROR 
;               -FFH TRANSMISSION ERROR
; CALLS      : CI,INHEX1,INHEX2 
; DESTROYS   : A,C,E,H,L,F/F
; DESCRIPTION: RECEIVES A HEX FILE AND PUTS IT IN THE MEMORY.
;              IF A TRANSMISSION ERROR OCCURS, RETURNS WITH AN 
;              ERROR CODE IN REG A.

GETFL:
    MVI A,GOOD_CODE ; Assume that you don't have a transmission error.
    STA TEMP        ; Save code for later use.
GETF1:
    LDA DPORT       ; Find which port to use.
    CPI 01 
    JNZ GETF11
    CALL CI         ; Get an ASCII character from DB9-1 port.
    JMP GETF13
GETF11:
    IN UART_CTL_ST  ; Read console status.
    ANI UART_RBR    ; Check for receiver buffer ready.
    JZ GETF12       ; No character received.
    
    IN UART_DATA    ; A character is received. Read the character.
    CPI CTRL_C      ; Is it a Ctrl-C character.
    JZ ERROR        ; Yes - Character is a Ctrl-C - Stop and exit.
GETF12:
    CALL SID_IN     ; Get an ASCII character from DB9-2 port.
    CPI ':'         ; Is it ":"?
    JNZ GETF1       ; No - Wait for ,":".
    JMP GETF14
GETF13:
    CPI CTRL_C      ; Is it a Ctrl-C character.
    JZ ERROR        ; Yes - Character is a Ctrl-C - Stop and exit.
    CPI ':'         ; Is it ":"?
    JNZ GETF1       ; No - Wait for
GETF14:
    MVI C,00        ; Initialize checksum counter.
    CALL INHEX1     ; Get 2 ASCII characters and convert them in one byte.
    CPI 00          ; Is the count of bytes in the message 0 ?
    JZ GETF3        ; Yes - ok you have finished.
    MOV E,A         ; Save the number of bytes in reg E.
    ADD C           ; Start computing the checksum.
    MOV C,A         ; The checksum in reg C.
    CALL INHEX2     ; Get 2 bytes. It's the loading address.
    MOV A,H         ; Add this 2 bytes with checksum.
    ADD C           ; These 2 bytes are in reg HL.
    MOV C,A
    MOV A,L         ; Add the second byte.
    ADD C
    MOV C,A         ; Checksum in reg C.
    CALL INHEX1     ; Get a byte. Ignore it (record type).
    ADD C           ; It is valid thou for checksum.
    MOV C,A         ; Put it back to reg C.
GETBT:              ; Get a byte and compute the checksum.
    CALL INHEX1     ; Get a byte.
    MOV M,A         ; Put in memory.
    ADD C           ; Add it to checksum.
    MOV C,A
    INX H           ; Point at the next memory location.
    DCR E           ; Got all the bytes in this record ?
    JNZ GETBT       ; Not yet, go get the next.
    CALL INHEX1     ; Get checksum byte.
    ADD C           ; Add the checksum byte.
    JNZ GETF2       ; If the sum is not 00 put error code in TEMP.
    JMP GETF1       ; Go get the next record.
GETF2:
    MVI A,ERR_CODE  ; Code for transmission error.
    STA TEMP
    JMP GETF1       ; Go get the next record.
GETF3:
    LDA TEMP        ; Get transmission code in register A.
    RET
    
;***************************************************************************

; FUNCTION   : INHEX1 - GETS 2 ASCII CHARACTERS 
; INPUTS     : NONE
; OUTPUTS    : REG A - THE RECEIVED BYTE 
; CALLS      : CI,HEX_BIN 
; DESTROYS   : A,C,D,F/F
; DESCRIPTION: THE ROUTINE GETS 2 ASCII CHARACTERS AND CONVERTS THEM TO 
;              1 BYTE. IT DOES NOT CHECK FOR VALID ASCII CODE.

INHEX1:
    LDA DPORT        ; Find which port to use.
    CPI 01
    JNZ INHEX11
    CALL CI          ; Get the first ASCII from DB9-1 port.
    JMP INHEX12
INHEX11:
    CALL SID_IN      ; Get the first ASCII from DB9-2 port.
INHEX12:
    ANI 7FH          ; Mask MSB. Want to have always MSB=0.
    CALL HEX_BIN     ; Convert ASCII to 4-bit binary.
    RLC
    RLC
    RLC              ; Rotate the bits to hight order bits
    RLC              ; of the byte.
    ANI 0F0H         ; Mask low nibble.
    MOV D,A          ; Store the 4 bits at reg D.
    
    LDA DPORT        ; Find which port to use.
    CPI 01
    JNZ INHEX13
    CALL CI          ; Get the second ASCII from DB9-1 port.
    JMP INHEX14
INHEX13:
    CALL SID_IN      ; Get the second ASCII from DB9-2 port.
INHEX14:
    ANI 7FH          ; Mask MSB.
    CALL HEX_BIN     ; Convert to 4 bit binary.
    ANI 0FH          ; Mask high nibble.
    ORA D            ; Put together the nibbles at reg A.
    RET              ; All done.
    
;***************************************************************************

; FUNCTION   : INHEX2 - GETS 4 ASCII
; DESCRIPTION: THE ROUTINE GETS 4 ASCII AND CONVERTS 
;              THEM IN 2 BYTES. PUTS THE RESULT IN HL REG.

INHEX2:
    CALL INHEX1      ; Get the first 2 ASCII.
    MOV H,A          ; Put them in H reg.
    CALL INHEX1      ; Get the second 2 ASCII.
    MOV L,A          ; Put them in L reg.
    RET              ; All done.
    
;***************************************************************************

; FUNCTION   : SID_IN - INPUT FROM SERIAL PORT DB9-2.
; INPUTS     : A CHARACTER FROM SERIAL PORT DB9-2.
; OUTPUTS    : REG A - THE CHARACTER RECEIVED.
; CALLS      : NONE 
; DESTROYS   : A, F/F
; DESCRIPTION: THE ROUTINE SCANS SID PIN OF CPU WITH PROPER 
;              RATE AND CONVERTS THE SERIAL INCOMING BITS
;              TO BYTE. THIS IS A SOFTWARE UART. THE TIMING OF
;              THIS ROUTING IS CRITICAL FOR ESTABLISHING
;              THE PROPER BAUD RATE. THE TIMING IS CONTROLED
;              BY SOFTWARE LOOPS.

SID_IN:
    DI               ; All instructions in this routine are critical
    PUSH B           ; for the baud rate timing.
    PUSH H           ; Save CPU registers.
    MVI B,NBITS      ; Number of data bits forming a byte.
CI1:
    RIM              ; Wait for start bit.
    ORA A            ; Set flags.
    JM CI1
    LHLD HLBIT       ; Value of half bit time in HL.
CI2:
    DCR L            ; Wait until middle of start bit.
    JNZ CI2
    DCR H
    JNZ CI2
CI3:
    LHLD BTIME       ; Value for one bit in HL.
CI4:
    DCR L            ; Wait until the next bit.
    JNZ CI4
    DCR H
    JNZ CI4
    RIM              ; Read the bit from serial port.
    RAL              ; Data bit in register C.
    DCR B            ; Check if this is the last bit.
    JZ CI5           ; Yes - Ready to return.
    MOV A,C          ; No - Put character in reg. A.
    RAR              ; Insert new bit.
    MOV C,A          ; Put it back in reg. C.
    NOP              ; Timing instruction.
    JMP CI3
CI5:
    MOV A,C          ; Put the byte just received in reg. A.
    POP H            ; Restore cpu registers.
    POP B
    EI
    RET              ; Character complete - Return.
    
;***************************************************************************

; FUNCTION   : OUTDS - DISPLAY OUTPUT 
; INPUTS     : NOTHING
; OUTPUTS    : WRITES CHARACTERS TO MICROKIT'S DISPLAY 
; CALLS      : NONE 
; DESTROYS   : A,C,H,L,F/F
; DESCRIPTION: THE ROUTINE OUTPUTS A MESSAGE AT THE MICROKIT'S DISPLAY. 

OUTDS:
    MVI C,04H        ; Number of characters to display.
    MVI A,90H        ; Control byte to 8279.
    OUT CNTRL
OTDS1:
    MOV A,M          ; Get character from memory.
    CMA              ; Complement the character.
    OUT DSPLY        ; Output to display.
    INX H            ; Next character.
    DCR C            ; Decrement character counter.
    JNZ OTDS1        ; Is it the last character? - No get next.
    RET              ; Yes - All done.
    
;*************************************************************************** 
;            DOWNLOAD PROGRAM PARAMETER SETTINGS

HLBIT: DB 09H,01H    ; Wait value for half bit time. 9600 bits/sec.
BTIME: DB 12H,01H    ; Wait value for one bit time. 9600 bits/sec.

;*************************************»************************************* 
;            DOWNLOAD PROGRAM MESSAGES TO MICROKIT'S DISPLAY

DMES: DB 83H,0F3H,77H,0E5H   ; Message "LOAd"
ENDM: DB 97H,45H,0E5H,00H    ; Message "End"
BADM: DB 0C7H,77H,0E5H,00H   ; Message "bAd"

;***************************************************************************
;            DOWNLOAD PROGRAM MESSAGES TO CRT SCREEN

LOAD_MES1:      DB CR,LF," LOADING A HEX FILE FROM PORT DB9-",00

LOAD_MES2:      DB CR,LF," PLEASE WAIT ... ",00

LOAD_END_MES:   DB CR,LF," END OF TRANSMISSION",CR,LF,00

LOAD_ERROR_MES: DB CR,LF," TRANSMISSION ERROR",CR,LF,00

;***************************************************************************
;
;                     SELECT PORT FOR DOWNLOAD
;
;****************************************************************************

; FUNCTION   : KEYBR_PORT_CMD - KEYBOARD MONITOR SELECT DOWNLOAD PORT ROUTINE 
; INPUTS     : NONE
; OUTPUTS    : NONE
; CALLS      : CLEAR, OUTPT, GETFL
; DESTROYS   : A,B,C,D,E,H,L,F/F
; DESCRIPTION: THIS IS THE MAIN ROUTINE THAT CHANGES THE DOWNLOAD 
;              PORT WHEN KEYBOARD MONITOR IS USED.


    ORG 01800H       ; The program starts at this location.
    
KEYBR_PORT_CMD:
    MVI A,UNMSK      ; Unmask interrupts from the keyboard.
    SIM
    
    MVI B,NODOT      ; ARG - No dot in address field of display.
    CALL CLEAR       ; Clear display.
    LXI H,PMES1      ; Display message 11 POrt
    MVI A,ADFLD      ; Use address field of display.
    MVI B,NODOT      ; No dot on display.
    CALL OUTPT       ; Display message.
PORT_1:
    LDA DPORT        ; Get active port number.
    STA CURDT        ; Store port number at current data.
    MVI B,DOT        ; ARG - Dot in data field.
    CALL UPDDT       ; Update data field of display.
    
    CALL RDKBD       ; Wait for the user's answer.
    CPI PERIOD
    JZ PORT_EXIT
    CPI 1
    JNZ PORT_2
    STA DPORT
    JMP PORT_1
PORT_2:
    CPI 2 
    JNZ PORT_1
    STA DPORT
    JMP PORT_1
PORT_EXIT:
    MVI B,NODOT      ; ARG - No dot in address field of display.
    CALL CLEAR       ; Clear display.
    JMP CMMND        ; Give control to keyboard monitor.
    
;*************************************************************************** 
;            SELECT PORT ROUTINE MESSAGES TO MICROKIT'S DISPLAY

PMES1: DB LETRP,ZERO,LETRR,LETRT ; "POrt"

;*************************************************************************** 
;            SELECT PORT MESSAGES TO CRT SCREEN

DPORT_MES1: DB CR,LF," ACTIVE PORT FOR LOAD COMMAND IS DB9-",00

DPORT_MES2: DB CR,LF," PRESS <CR> TO EXIT WITHOUT ALTERING",CR,LF 
    DB " PRESS 1 OR 2 TO SELECT NEW PORT.  SELECT : ",00
    
;***************************************************************************

COPYRIGHT DB " MICRO DESIGN LTD  1985,1991 "

;****************************************************************************
;****************************************************************************
;
;                             RAM USAGE
;
;****************************************************************************
;****************************************************************************

; THE SPACE 4000H-40D0H IS RESERVED FOR THE MONITOR AND USER STACK.


    ORG MONSTK       ; Start of monitor stack.
    
MNSTK:
    ORG USERVR
    
;****************************************************************************

; IN THE FOLLOWING 6 LOCATIONS THE USER CAN PLACE VARIABLES OF 
; HIS PROGRAM.

STADR:  DS 2         ; Location in RAM used for saving start address.
EDADR:  DS 2         ; Location in RAM used for saving end address.
CHKSUM: DS 1         ; Location in RAM used for saving checksum.
        DS 1

;****************************************************************************

; IN THE FOLLOWING LOCATIONS, THE USER MAY PLACE JUMP 
; INSTRUCTIONS TO ROUTINES FOR HANDLING THE FOLLOWING:
;        A) RST 5,6 & 7 INSTRUCTIONS
;        B) HARDWIRED USER INTERRUPT (RST 6.5)
;        C) KEYBOARD "VECTORED INTERRUPT" KEY (RSRT 7.5)

    ORG USERBR       ; Start of user branch locations.
    
USRBR:
    RSET5: DB 0,0,0  ; Jump to RST 5 routine.
    RSET6: DB 0,0,0  ; Jump to RST 6 routine.
    RST65: DB 0,0,0  ; Jump to RST 6.5 routine (hardwired user interrupt).
    RSET7: DB 0,0,0  ; Jump to RST 7 routine.
    USINT: DB 0,0,0  ; Jump to "VECTORED INTERRUPT" key routine.

;***************************************************************************

; SAVE LOCATIONS FOR USER REGISTERS.

USRRG:
ESAV:  DB 0          ; E register.
DSAV:  DB 0          ; d register.
CSAV:  DB 0          ; C register.
BSAV:  DB 0          ; B register.
FSAV:  DB 0          ; Flags.
ASAV:  DB 0          ; A register.
LSAV:  DB 0          ; L register.
HSAV:  DB 0          ; H register.
ISAV:  DB 0          ; Interrupt mask.
PSAV:                ; Program counter.
PCLSV: DB 0          ; Low order byte of program counter.
PCHSV: DB 0          ; High order BYTE of program counter.
SSAV:                ; stack pointer.
SPLSV: DB 0          ; Low order byte of stack pointer.
SPHSV: DB 0          ; High order byte of stack pointer.

;***************************************************************************

; MONITOR STORAGE LOCATIONS.


PORT:  DS 3           ; Locations for storing operation codes for two
                     ; instructions. Used in IN_CMD and OUT_CMD.
DPORT: DS 1          ; Save location for active download port (1 or 2).
AIMAG: DS 1          ; Image of register A.
CURAD: DS 2          ; Current address.
CURDT: DS 1          ; Current data.
OBUFF: DS 4          ; Output buffer.
TEMP:  DS 1          ; Temporary location for sigle step routine
                     ; or for CRT monitor.
RGPTR: DS 1          ; Register pointer.
IBUFF: DS 1          ; Input buffer.
SVCMC: DS 1          ; Location for saving the command code.
USCSR: DS 1          ; User should store image of CSR here each time
                     ; CSR is changed. Otherwise, single step routine 
                     ; will destroy CSR contents. The CSR is the coimiand 
                     ; status/register of the 8155.

;****************************************************************************

END
