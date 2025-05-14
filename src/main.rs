#![no_std]
#![no_main]
#![feature(offset_of)]

use core::fmt::Write;
use core::panic::PanicInfo;
use core::time::Duration;
use mustard::error;
use mustard::executor::Executor;
use mustard::executor::Task;
use mustard::executor::TimeoutFuture;
use mustard::graphics::BitmapTextWriter;
use mustard::hpet::global_timestamp;
use mustard::info;
use mustard::init::init_allocator;
use mustard::init::init_basic_runtime;
use mustard::init::init_display;
use mustard::init::init_hpet;
use mustard::init::init_paging;
use mustard::print::hexdump;
use mustard::println;
use mustard::qemu::exit_qemu;
use mustard::qemu::QemuExitCode;
use mustard::uefi::init_vram;
use mustard::uefi::locate_loaded_image_protocol;
use mustard::uefi::EfiHandle;
use mustard::uefi::EfiSystemTable;
use mustard::warn;
use mustard::x86::init_exceptions;

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
    init_display(&mut vram);
    let mut w = BitmapTextWriter::new(&mut vram);
    let acpi = efi_system_table.acpi_table().expect("ACPI table not found");
    let memory_map = init_basic_runtime(image_handle, efi_system_table);
    writeln!(w, "Hello, Non-UEFI world!").unwrap();
    init_allocator(&memory_map);
    let (_gdt, _idt) = init_exceptions();
    init_paging(&memory_map);
    init_hpet(acpi);
    let t0 = global_timestamp();
    let task1 = Task::new(async move {
        for i in 100..=103 {
            info!("{i} hpet.main_counter = {:?}", global_timestamp() - t0);
            TimeoutFuture::new(Duration::from_secs(1)).await;
        }
        Ok(())
    });
    let task2 = Task::new(async move {
        for i in 200..=203 {
            info!("{i} hpet.main_counter = {:?}", global_timestamp() - t0);
            TimeoutFuture::new(Duration::from_secs(2)).await;
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
