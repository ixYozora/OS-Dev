; ╔═════════════════════════════════════════════════════════════════════════╗
; ║ Module: boot                                                            ║
; ╟─────────────────────────────────────────────────────────────────────────╢
; ║ Descr.: grub loads our image at the address 1 MB and switches to 32 bit ║
; ║         protected mode and jumps to the first function 'start' in this  ║
; ║         file. We switch to 64 bit long mode and call 'startup', the     ║
; ║         first rust function.                                            ║
; ╟─────────────────────────────────────────────────────────────────────────╢
; ║ Author: Michael Schoettner, Univ. Duesseldorf, 26.2.2023                ║
; ╚═════════════════════════════════════════════════════════════════════════╝

;
;   Konstanten
;

; Auskommentieren, um im Grafikmodus zu booten
;%define TEXT_MODE

 
; Lade-Adresse des Kernels, muss mit der Angabe in 'sections' konsistent sein!
KERNEL_START: equ 0x100000


; Stack fuer die main-Funktion
STACKSIZE: equ 65536

; 254 GB maximale RAM-Groesse fuer die Seitentabelle
MAX_MEM: equ 254

; Speicherplatz fuer die Seitentabelle
[GLOBAL pagetable_start]
pagetable_start:  equ 0x103000    ; 1 MB + 12 KB

[GLOBAL pagetable_end]
pagetable_end:  equ 0x200000      ;  = 2 MB

;
;   System
;

; Von uns bereitgestellte Funktionen
[GLOBAL start]
[GLOBAL idt]
[GLOBAL _tss_set_base_address]
[GLOBAL _tss_set_rsp0]
[GLOBAL _kernel_rsp0]

; C-Funktion die am Ende des Assembler-Codes aufgerufen werden
[EXTERN startup]


; Vom Compiler bereitgestellte Adressen
[EXTERN ___BSS_START__]
[EXTERN ___BSS_END__]

; In 'sections' definiert
[EXTERN ___KERNEL_DATA_START__]
[EXTERN ___KERNEL_DATA_END__]

; Multiboot constants
MULTIBOOT_HEADER_MAGIC:           equ 0x1BADB002
MULTIBOOT_ARCHITECTURE_I386:      equ 0
MULTIBOOT_HEADER_TAG_OPTIONAL:    equ 1
MULTIBOOT_HEADER_TAG_FRAMEBUFFER: equ 5
MULTIBOOT_HEADER_TAG_END:         equ 0

MULTIBOOT_MEMORY_INFO	equ	1<<1
MULTIBOOT_GRAPHICS_INFO equ 1<<2

MULTIBOOT_HEADER_FLAGS	equ	MULTIBOOT_MEMORY_INFO | MULTIBOOT_GRAPHICS_INFO
MULTIBOOT_HEADER_CHKSUM	equ	-(MULTIBOOT_HEADER_MAGIC + MULTIBOOT_HEADER_FLAGS)

%ifdef TEXT_MODE
   MULTIBOOT_GRAPHICS_MODE    equ 1
   MULTIBOOT_GRAPHICS_WIDTH   equ 80
   MULTIBOOT_GRAPHICS_HEIGHT  equ 25
   MULTIBOOT_GRAPHICS_BPP     equ 0

%else
   MULTIBOOT_GRAPHICS_MODE   equ 0
   MULTIBOOT_GRAPHICS_WIDTH  equ 800
   MULTIBOOT_GRAPHICS_HEIGHT equ 600
   MULTIBOOT_GRAPHICS_BPP    equ 32
%endif

[SECTION .text]

;
;   System-Start, Teil 1 (im 32-bit Protected Mode)
;
;   Initialisierung von GDT und Seitentabelle und Wechsel in den 64-bit
;   Long Mode.
;

[BITS 32]

multiboot_header:
	align  4

;
;   Multiboot-Header zum Starten mit GRUB oder QEMU (ohne BIOS)
;
	dd MULTIBOOT_HEADER_MAGIC
	dd MULTIBOOT_HEADER_FLAGS
	dd -(MULTIBOOT_HEADER_MAGIC + MULTIBOOT_HEADER_FLAGS)
	dd multiboot_header   
	dd (___KERNEL_DATA_START__   - KERNEL_START)
	dd (___KERNEL_DATA_END__     - KERNEL_START)
	dd (___BSS_END__        - KERNEL_START)
	dd (startup             - KERNEL_START)
	dd MULTIBOOT_GRAPHICS_MODE
	dd MULTIBOOT_GRAPHICS_WIDTH
	dd MULTIBOOT_GRAPHICS_HEIGHT
	dd MULTIBOOT_GRAPHICS_BPP

;  GRUB Einsprungspunkt
start:
	cld              ; GCC-kompilierter Code erwartet das so
	cli              ; Interrupts ausschalten
	lgdt   [gdt_80]  ; Neue Segmentdeskriptoren setzen

	; Globales Datensegment
	mov    eax, 3 * 0x8
	mov    ds, ax
	mov    es, ax
	mov    fs, ax
	mov    gs, ax

	; Stack festlegen
	mov    ss, ax
	mov    esp, init_stack+STACKSIZE
   
	; Sichere Adresse der Multiboot-Struktur (ist in EBX)
	; da wird den Inhalt erst im 64 Bit Mode wieder herunterholen
	; muessen wir 8 Bytes 'pushen'
    push   0
    push   ebx

	jmp    init_longmode


;
;  Umschalten in den 64 Bit Long-Mode
;
init_longmode:
	; Adresserweiterung (PAE) aktivieren
	mov    eax, cr4
	or     eax, 1 << 5
	mov    cr4, eax

	; Seitentabelle anlegen (Ohne geht es nicht)
	call   setup_paging

	; Long-Mode (fürs erste noch im Compatibility-Mode) aktivieren
	mov    ecx, 0x0C0000080 ; EFER (Extended Feature Enable Register) auswaehlen
	rdmsr
	or     eax, 1 << 8 ; LME (Long Mode Enable)
	wrmsr

	; Paging aktivieren
	mov    eax, cr0
	or     eax, 1 << 31
	mov    cr0, eax

	; Sprung ins 64 Bit-Codesegment -> Long-Mode wird vollständig aktiviert
	jmp    2 * 0x8 : longmode_start


;
;   Anlegen einer (provisorischen) Seitentabelle mit 2 MB Seitengröße, die die
;   ersten MAX_MEM GB direkt auf den physikalischen Speicher abbildet.
;   Dies ist notwendig, da eine funktionierende Seitentabelle für den Long-Mode
;   vorausgesetzt wird. Mehr Speicher darf das System im Moment nicht haben.
;
setup_paging:
	; PML4 (Page Map Level 4 / 1. Stufe)
	mov    eax, pdp
	or     eax, 0xf
	mov    dword [pml4+0], eax
	mov    dword [pml4+4], 0

	; PDPE (Page-Directory-Pointer Entry / 2. Stufe) für aktuell 16GB
	mov    eax, pd
	or     eax, 0x7           ; Adresse der ersten Tabelle (3. Stufe) mit Flags.
	mov    ecx, 0
fill_tables2:
	cmp    ecx, MAX_MEM       ; MAX_MEM Tabellen referenzieren
	je     fill_tables2_done
	mov    dword [pdp + 8*ecx + 0], eax
	mov    dword [pdp + 8*ecx + 4], 0
	add    eax, 0x1000        ; Die Tabellen sind je 4kB groß
	inc    ecx
	ja     fill_tables2
fill_tables2_done:

	; PDE (Page Directory Entry / 3. Stufe)
	mov    eax, 0x0 | 0x87    ; Startadressenbyte 0..3 (=0) + Flags
	mov    ebx, 0             ; Startadressenbyte 4..7 (=0)
	mov    ecx, 0
fill_tables3:
	cmp    ecx, 512*MAX_MEM   ; MAX_MEM Tabellen mit je 512 Einträgen füllen
	je     fill_tables3_done
	mov    dword [pd + 8*ecx + 0], eax ; low bytes
	mov    dword [pd + 8*ecx + 4], ebx ; high bytes
	add    eax, 0x200000      ; 2 MB je Seite
	adc    ebx, 0             ; Overflow? -> Hohen Adressteil inkrementieren
	inc    ecx
	ja     fill_tables3
fill_tables3_done:

	; Basiszeiger auf PML4 setzen
	mov    eax, pml4
	mov    cr3, eax
	ret

;
;   System-Start, Teil 2 (im 64-bit Long-Mode)
;
;   Das BSS-Segment wird gelöscht und die IDT die PICs initialisiert.
;   Anschließend werden die Konstruktoren der globalen C++-Objekte und
;   schließlich main() ausgeführt.
;
longmode_start:
[BITS 64]
    ; zuvor gesicherter Zeiger auf multiboot infos vom Stack holen und
    ; in 'multiboot_info_address' sichern. Durch die Konstruktoren wird 
    ; der Stack manipuliert, daher muessen wir das gleich hier machen
    pop    rax  
    mov    [multiboot_info_address], rax
    
	; BSS löschen
	mov    rdi, ___BSS_START__
clear_bss:
	mov    byte [rdi], 0
	inc    rdi
	cmp    rdi, ___BSS_END__
	jne    clear_bss

;	fninit         ; FPU aktivieren

	; TSS-Basisadresse in GDT-Deskriptor eintragen
	call   _tss_set_base_address

	; Task-Register laden (TSS-Selektor = 0x30)
	mov    ax, 0x30
	ltr    ax

    mov    rdi, [multiboot_info_address] ; 1. Parameter wird in rdi uebergeben
	call   startup ; multiboot infos auslesen und 'main' aufrufen
	
	cli            ; Hier sollten wir nicht hinkommen
	hlt



;
; TSS-Basisadresse in den TSS-Deskriptor der GDT schreiben
;
_tss_set_base_address:
	mov    rax, _tss
	mov    [_tss_descriptor + 2], ax       ; Basis[15:0]
	shr    rax, 16
	mov    [_tss_descriptor + 4], al       ; Basis[23:16]
	shr    rax, 8
	mov    [_tss_descriptor + 7], al       ; Basis[31:24]
	shr    rax, 8
	mov    [_tss_descriptor + 8], eax      ; Basis[63:32]
	ret

;
; Kernel-Stack-Zeiger (rsp0) im TSS setzen
; Parameter: rdi = neuer rsp0-Wert
;
_tss_set_rsp0:
	mov    [_tss + 4], rdi
	mov    [_kernel_rsp0], rdi
	ret

;
; Kurze Verzögerung für in/out-Befehle
;
delay:
	jmp    .L2
.L2:
	ret


;
; Funktionen für den C++ Compiler. Diese Label müssen für den Linker
; definiert sein; da bei OOStuBS keine Freigabe des Speichers erfolgt, können
; die Funktionen aber leer sein.
;
__cxa_pure_virtual: ; "virtual" Methode ohne Implementierung aufgerufen
;_ZdlPv:             ; void operator delete(void*)
;_ZdlPvj:            ; void operator delete(void*, unsigned int) fuer g++ 6.x
;_ZdlPvm:            ; void operator delete(void*, unsigned long) fuer g++ 6.x
	ret


[SECTION .data]

;
; Segment-Deskriptoren
;
gdt:
	dw  0,0,0,0   ; NULL-Deskriptor

	; 32-Bit-Codesegment-Deskriptor (Selektor 0x08)
	dw  0xFFFF    ; 4Gb - (0x100000*0x1000 = 4Gb)
	dw  0x0000    ; base address=0
	dw  0x9A00    ; code read/exec
	dw  0x00CF    ; granularity=4096, 386 (+5th nibble of limit)

	; 64-Bit-Codesegment-Deskriptor (Selektor 0x10, Ring 0)
	dw  0xFFFF    ; 4Gb - (0x100000*0x1000 = 4Gb)
	dw  0x0000    ; base address=0
	dw  0x9A00    ; code read/exec
	dw  0x00AF    ; granularity=4096, 386 (+5th nibble of limit), Long-Mode

	; Datensegment-Deskriptor (Selektor 0x18, Ring 0)
	dw  0xFFFF    ; 4Gb - (0x100000*0x1000 = 4Gb)
	dw  0x0000    ; base address=0
	dw  0x9200    ; data read/write
	dw  0x00CF    ; granularity=4096, 386 (+5th nibble of limit)

	; User-Datensegment-Deskriptor (Selektor 0x20, Ring 3)
	; Placed before User Code for syscall/sysret compatibility:
	; sysret CS = STAR[63:48]+16 = 0x28, SS = STAR[63:48]+8 = 0x20
	dw  0xFFFF
	dw  0x0000
	dw  0xF200    ; P=1, DPL=3, S=1, Type=0010 (data read/write)
	dw  0x00CF

	; 64-Bit User-Codesegment-Deskriptor (Selektor 0x28, Ring 3)
	dw  0xFFFF
	dw  0x0000
	dw  0xFA00    ; P=1, DPL=3, S=1, Type=1010 (code exec/read)
	dw  0x00AF    ; Long-Mode

	; TSS-Deskriptor (Selektor 0x30, doppelte Groesse = 16 Bytes)
_tss_descriptor:
	dw  0x0067    ; Limit[15:0] = 103 (TSS-Groesse - 1)
	dw  0x0000    ; Basis[15:0]  (wird zur Laufzeit gesetzt)
	db  0x00      ; Basis[23:16] (wird zur Laufzeit gesetzt)
	db  0x89      ; P=1, DPL=0, S=0, Type=1001 (64-Bit TSS, verfuegbar)
	db  0x00      ; G=0, Limit[19:16]=0
	db  0x00      ; Basis[31:24] (wird zur Laufzeit gesetzt)
	dd  0x00000000 ; Basis[63:32] (wird zur Laufzeit gesetzt)
	dd  0x00000000 ; Reserviert

gdt_80:
	dw  gdt_80 - gdt - 1   ; GDT Limit
	dq  gdt                 ; Adresse der GDT

multiboot_info_address:
	dq  0

; Cached copy of TSS RSP0 for fast syscall entry (avoids offset into TSS struct)
_kernel_rsp0:
	dq  0

; Task State Segment (104 Bytes, ohne IO-Bitmap)
align 8
_tss:
	dd  0               ; Offset 0x00: Reserviert
	dq  0               ; Offset 0x04: RSP0
	dq  0               ; Offset 0x0C: RSP1
	dq  0               ; Offset 0x14: RSP2
	dq  0               ; Offset 0x1C: Reserviert
	dq  0               ; Offset 0x24: IST1
	dq  0               ; Offset 0x2C: IST2
	dq  0               ; Offset 0x34: IST3
	dq  0               ; Offset 0x3C: IST4
	dq  0               ; Offset 0x44: IST5
	dq  0               ; Offset 0x4C: IST6
	dq  0               ; Offset 0x54: IST7
	dq  0               ; Offset 0x5C: Reserviert
	dw  0               ; Offset 0x64: Reserviert
	dw  104             ; Offset 0x66: IO-Map-Basis = TSS-Groesse (keine IO-Bitmap)

[SECTION .bss]

global init_stack:data (init_stack.end - init_stack)
init_stack:
	resb STACKSIZE
.end:


;
; Speicher fuer Page-Tables
;
[SECTION .global_pagetable]

[GLOBAL pml4]
[GLOBAL pdp]
[GLOBAL pd]

pml4:
    times 4096 db 0
	alignb 4096

pd:
    times MAX_MEM*4096 db 0
	alignb 4096

pdp:
    times MAX_MEM*8 db 0    ; 254*8 = 2032

