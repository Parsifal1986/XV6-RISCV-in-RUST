OUTPUT_ARCH( "riscv" )
ENTRY( _start )

/*
 * xv6's exec() requires each loadable segment to start
 * on a page boundary, so put text and read-only data in
 * one segment, and data and bss in another.
 */
PHDRS
{
  text PT_LOAD FLAGS(5); /* R X */
  data PT_LOAD FLAGS(6); /* R W */
}

SECTIONS
{
  . = 0x0;

  .text : {
    *(.text.entry)
    *(.text .text.*)
  } :text

  .rodata : {
    . = ALIGN(16);
    *(.srodata .srodata.*) /* do not need to distinguish this from .rodata */
    . = ALIGN(16);
    *(.rodata .rodata.*)
  } :text

  .eh_frame : {
    *(.eh_frame)
    *(.eh_frame.*)
  } :text

  . = ALIGN(0x1000);

  .data : {
    . = ALIGN(16);
    *(.sdata .sdata.*) /* do not need to distinguish this from .data */
    . = ALIGN(16);
    *(.data .data.*)
  } :data

  .bss : {
    . = ALIGN(16);
    *(.sbss .sbss.*) /* do not need to distinguish this from .bss */
    . = ALIGN(16);
    *(.bss .bss.*)
  } :data

  PROVIDE(end = .);
}
