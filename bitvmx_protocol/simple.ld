SECTIONS
{
    . = 0x80000000;
    .text : { *(.text) }
    
    . = 0x90000000;
    .data : { *(.data) }
    
    . = 0xa0000000;
    .bss : { *(.bss) }
}