#![no_std]
#![no_main]
#![feature(offset_of)]

use core::fmt::Write;
use core::panic::PanicInfo;
use mustard::error;
use mustard::graphics::draw_test_pattern;
use mustard::graphics::fill_rect;
use mustard::graphics::Bitmap;
use mustard::info;
use mustard::init::init_basic_runtime;
use mustard::print::hexdump;
use mustard::println;
use mustard::qemu::exit_qemu;
use mustard::qemu::QemuExitCode;
use mustard::uefi::init_vram;
use mustard::uefi::EfiHandle;
use mustard::uefi::EfiMemoryType;
use mustard::uefi::EfiSystemTable;
use mustard::uefi::VramTextWriter;
use mustard::warn;
use mustard::x86::hlt;

#[no_mangle]
fn efi_main(image_handle: EfiHandle, efi_system_table: &EfiSystemTable) {
    println!("Booting MustardOS...");
    println!("image_handle: {:#018X}", image_handle);
    println!("efi_system_table: {:p}", efi_system_table);
    info!("info");
    warn!("warn");
    error!("error");
    hexdump(efi_system_table);
    let mut vram = init_vram(efi_system_table).expect("init_vram failed");
    let vw = vram.width();
    let vh = vram.height();
    fill_rect(&mut vram, 0x000000, 0, 0, vw, vh).expect("fill_rect failed");
    draw_test_pattern(&mut vram);
    let mut w = VramTextWriter::new(&mut vram);
    let memory_map = init_basic_runtime(image_handle, efi_system_table);
    let mut total_memory_pages = 0;
    for e in memory_map.iter() {
        if e.memory_type() != EfiMemoryType::CONVENTIONAL_MEMORY {
            continue;
        }
        total_memory_pages += e.number_of_pages();
        writeln!(w, "{e:?}").unwrap();
    }
    let total_memory_size_mib = total_memory_pages * 4096 / 1024 / 1024;
    writeln!(
        w,
        "Total: {total_memory_pages} pages = {total_memory_size_mib} MiB"
    )
    .unwrap();
    writeln!(w, "Hello, Non-UEFI world!").unwrap();
    let cr3 = mustard::x86::read_cr3();
    println!("cr3 = {cr3:#p}");
    let t = Some(unsafe { &*cr3 });
    println!("{t:?}");
    let t = t.and_then(|t| t.next_level(0));
    println!("{t:?}");
    let t = t.and_then(|t| t.next_level(0));
    println!("{t:?}");
    let t = t.and_then(|t| t.next_level(0));
    println!("{t:?}");

    loop {
        hlt()
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("PANIC: {info:?}");
    exit_qemu(QemuExitCode::Fail);
}
