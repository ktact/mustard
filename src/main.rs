#![no_std]
#![no_main]
#![feature(offset_of)]

use core::fmt::Write;
use core::panic::PanicInfo;
use mustard::error;
use mustard::executor::yield_execution;
use mustard::executor::Executor;
use mustard::executor::Task;
use mustard::graphics::draw_test_pattern;
use mustard::graphics::fill_rect;
use mustard::graphics::Bitmap;
use mustard::hpet::Hpet;
use mustard::info;
use mustard::init::init_basic_runtime;
use mustard::init::init_paging;
use mustard::print::hexdump;
use mustard::println;
use mustard::qemu::exit_qemu;
use mustard::qemu::QemuExitCode;
use mustard::uefi::init_vram;
use mustard::uefi::locate_loaded_image_protocol;
use mustard::uefi::EfiHandle;
use mustard::uefi::EfiMemoryType;
use mustard::uefi::EfiSystemTable;
use mustard::uefi::VramTextWriter;
use mustard::warn;
use mustard::x86::flush_tlb;
use mustard::x86::init_exceptions;
use mustard::x86::read_cr3;
use mustard::x86::trigger_debug_interrupt;
use mustard::x86::PageAttr;

#[no_mangle]
fn efi_main(image_handle: EfiHandle, efi_system_table: &EfiSystemTable) {
    println!("Booting MustardOS...");
    println!("image_handle: {:#018X}", image_handle);
    println!("efi_system_table: {:p}", efi_system_table);
    let loaded_image_protocol = locate_loaded_image_protocol(image_handle, efi_system_table)
        .expect("Failed to get LoadedImageProtocol");
    println!("image_base: {:#018X}", loaded_image_protocol.image_base);
    println!("image_size: {:#018X}", loaded_image_protocol.image_size);
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
    let acpi = efi_system_table.acpi_table().expect("ACPI table not found");
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

    let (_gdt, _idt) = init_exceptions();
    info!("Exception initialized!");
    trigger_debug_interrupt();
    info!("Execution continued.");
    init_paging(&memory_map);
    info!("Now we are using our own page tables!");

    let page_table = read_cr3();
    unsafe {
        (*page_table)
            .create_mapping(0, 4096, 0, PageAttr::NotPresent)
            .expect("Failed to unmap page 0");
    }
    flush_tlb();

    let hpet = acpi.hpet().expect("Failed to get HPET from ACPI");
    let hpet = hpet
        .base_address()
        .expect("Failed to get HPET base address");
    info!("HPET is at {hpet:#p}");
    let hpet = Hpet::new(hpet);
    let task1 = Task::new(async move {
        for i in 100..=103 {
            info!("{i} hpet.main_counter = {}", hpet.main_counter());
            yield_execution().await;
        }
        Ok(())
    });
    let task2 = Task::new(async {
        for i in 200..=203 {
            info!("{i}");
            yield_execution().await;
        }
        Ok(())
    });

    let mut executor = Executor::new();
    executor.enqueue(task1);
    executor.enqueue(task2);
    Executor::run(executor);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("PANIC: {info:?}");
    exit_qemu(QemuExitCode::Fail);
}
