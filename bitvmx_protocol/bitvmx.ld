/* BitVMX 링커 스크립트 */

MEMORY
{
    /* 프로그램 코드 */
    TEXT (rx) : ORIGIN = 0x80000000, LENGTH = 0x10000
    
    /* 데이터 */
    DATA (rw) : ORIGIN = 0x90000000, LENGTH = 0x10000
    
    /* BSS */
    BSS (rw) : ORIGIN = 0xa0000000, LENGTH = 0x10000
    
    /* 입력 버퍼 */
    INPUT (rw) : ORIGIN = 0xaa000000, LENGTH = 0x2000
    
    /* 스택 */
    STACK (rw) : ORIGIN = 0xe0000000, LENGTH = 0x800000
    
    /* 레지스터 */
    REGISTERS (rw) : ORIGIN = 0xf0000000, LENGTH = 0x200
}

SECTIONS
{
    .text : {
        *(.text._start)
        *(.text)
    } > TEXT
    
    .data : {
        *(.data)
    } > DATA
    
    .bss : {
        *(.bss)
    } > BSS
    
    .input : {
        . = ALIGN(4);
    } > INPUT
    
    .stack : {
        . = ALIGN(4);
    } > STACK
    
    .registers : {
        . = ALIGN(4);
    } > REGISTERS
}