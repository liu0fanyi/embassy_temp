use std::{env, path::PathBuf};

fn main() {
    // println!("cargo:rerun-if-changed=build.rs");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    // let ld = &out.join("rustsbi-prototyper.ld");
    let ld = &out.join("linker.ld");

    // 将LINKER_SCRIPT 内容写入 $OUT_DIR/rustsbi-prototyper.ld
    std::fs::write(ld, LINKER_SCRIPT).unwrap();

    // 如果这些环境变量变化，会重新运行构建脚本
    // println!("cargo:rerun-if-env-changed=RUST_LOG,PROTOTYPER_FDT,PROTOTYPER_IMAGE");
    // 通过 cargo:rustc-link-arg 告诉 Rust 使用这个链接脚本
    // 通过 cargo:rustc-link-search 添加链接搜索路径
    //
    // println!("cargo:rustc-link-arg=-nostartfiles");
    println!("cargo:rustc-link-arg=-T{}", ld.display());
    println!("cargo:rustc-link-search={}", out.display());
}

// OUTPUT_ARCH(riscv)：指定目标架构为 RISC-V
// ENTRY(_start)：程序入口点为 _start
// . = 0x80000000：代码加载地址（RISC-V 常见的启动地址）
// .text：代码段（.text.entry 是启动代码，后面是其他代码）
// .rodata：只读数据段
// .data：可读写数据段
// .bss：未初始化数据段（堆栈、堆等）
// .text 0x80200000：payload 段（U-Boot 或其他内核）的加载地址
// *(.section_name) 会收集所有标记为该段的目标代码
// Rust的#[link_section]就是将函数/数据放入指定段的标准方法
const LINKER_SCRIPT: &[u8] = b"OUTPUT_ARCH(riscv)
ENTRY(_start)

SECTIONS {
    . = 0x80400000;

    .text : { 
        *(.text.entry)
        *(.text .text.*)
    }
    .rodata : ALIGN(0x1000)  {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    .data : ALIGN(0x1000)  {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    .bss (NOLOAD) : ALIGN(0x1000) {
        *(.bss.stack)
        start_bss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        end_bss = .;
    }

    /DISCARD/ : {
        *(.eh_frame)
        *(.debug*)
    }
}";

// .bss : {
//     start_bss = .;
//     *(.bss.stack)
//     *(.sbss .sbss.*)
//     end_bss = .;
// }
