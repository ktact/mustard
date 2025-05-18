#![no_std]
#![no_main]
#![feature(offset_of)]

use core::panic::PanicInfo;
use core::time::Duration;
use mustard::error;
use mustard::executor::sleep;
use mustard::executor::spawn_global;
use mustard::executor::start_global_executor;
use mustard::hpet::global_timestamp;
use mustard::info;
use mustard::init::init_allocator;
use mustard::init::init_basic_runtime;
use mustard::init::init_display;
use mustard::init::init_hpet;
use mustard::init::init_paging;
use mustard::init::init_pci;
use mustard::print::hexdump;
use mustard::print::set_global_vram;
use mustard::println;
use mustard::qemu::exit_qemu;
use mustard::qemu::QemuExitCode;
use mustard::serial::SerialPort;
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
    set_global_vram(vram);
    let acpi = efi_system_table.acpi_table().expect("ACPI table not found");
    let memory_map = init_basic_runtime(image_handle, efi_system_table);
    init_allocator(&memory_map);
    let (_gdt, _idt) = init_exceptions();
    init_paging(&memory_map);
    init_hpet(acpi);
    init_pci(acpi);
    let t0 = global_timestamp();
    let task1 = async move {
        for i in 100..=103 {
            info!("{i} hpet.main_counter = {:?}", global_timestamp() - t0);
            sleep(Duration::from_secs(1)).await;
        }
        Ok(())
    };
    let task2 = async move {
        for i in 200..=203 {
            info!("{i} hpet.main_counter = {:?}", global_timestamp() - t0);
            sleep(Duration::from_secs(2)).await;
        }
        Ok(())
    };
    let serial_task = async {
        let sp = SerialPort::default();
        if let Err(e) = sp.loopback_test() {
            error!("{e:?}");
            return Err("serial: loopback test failed");
        }
        info!("Started to monitor serial port");
        loop {
            if let Some(v) = sp.try_read() {
                let c = char::from_u32(v as u32);
                info!("serial input: {v:#04X} = {c:?}");
            }
            sleep(Duration::from_millis(20)).await;
        }
    };

    spawn_global(task1);
    spawn_global(task2);
    spawn_global(serial_task);
    start_global_executor();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("PANIC: {info:?}");
    exit_qemu(QemuExitCode::Fail);
}
