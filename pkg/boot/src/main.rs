#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

#[macro_use]
extern crate log;
extern crate alloc;

// use alloc::boxed::Box;
// use alloc::vec;
use elf::{load_elf, map_physical_memory, map_range};
use uefi::{entry, mem::memory_map::MemoryMap, Status};
use x86_64::{registers::control::*};
use ysos_boot::{config::Config, *};

mod config;

const CONFIG_PATH: &str = "\\EFI\\BOOT\\boot.conf";

#[entry]
fn efi_main() -> Status {
    uefi::helpers::init().expect("Failed to initialize utilities");

    log::set_max_level(log::LevelFilter::Info);
    info!("Running UEFI bootloader...");

    // 1. Load config
    let mut file = fs::open_file("\\EFI\\BOOT\\boot.conf");
    let buf = fs::load_file(&mut file);
    let config = {
        /* FIXME: Load config file as Config */
        Config::parse(&buf)
    };

    info!("Config: {:#x?}", config);

    // 2. Load ELF files
    let mut file = fs::open_file(config.kernel_path);
    let buf = fs::load_file(&mut file);
    let elf = {
        /* FIXME: Load kernel elf file */
        let elf_file = match xmas_elf::ElfFile::new(&buf) {
            Ok(res) => res,
            Err(e) => {
                panic!("{}", e)
            }
        };
        elf_file
    };

    unsafe {
        set_entry(elf.header.pt2.entry_point() as usize);
    }

    // 3. Load MemoryMap
    let mmap = uefi::boot::memory_map(MemoryType::LOADER_DATA).expect("Failed to get memory map");

    let max_phys_addr = mmap
        .entries()
        .map(|m| m.phys_start + m.page_count * 0x1000)
        .max()
        .unwrap()
        .max(0x1_0000_0000); // include IOAPIC MMIO area

    // 4. Map ELF segments, kernel stack and physical memory to virtual memory
    let mut page_table = current_page_table();

    // FIXME: root page table is readonly, disable write protect (Cr0)
    unsafe {
        Cr0::update(|f|{
            f.remove(Cr0Flags::WRITE_PROTECT);
        });
    }

    // FIXME: map physical memory to specific virtual address offset
    map_physical_memory(config.physical_memory_offset, max_phys_addr, &mut page_table, &mut UEFIFrameAllocator);

    // FIXME: load and map the kernel elf file
    load_elf(&elf, config.physical_memory_offset, &mut page_table, &mut UEFIFrameAllocator).expect("load elf error");

    // FIXME: map kernel stack
    map_range(config.kernel_stack_address, config.kernel_stack_size, &mut page_table, &mut UEFIFrameAllocator).expect("map stack error");

    // FIXME: recover write protect (Cr0)
    unsafe {
        Cr0::update(|f|{
            f.insert(Cr0Flags::WRITE_PROTECT);
        });
    }

    free_elf(elf);

    // 5. Pass system table to kernel
    let ptr = uefi::table::system_table_raw().expect("Failed to get system table");
    let system_table = ptr.cast::<core::ffi::c_void>();

    // 6. Exit boot and jump to ELF entry
    info!("Exiting boot services...");

    let mmap = unsafe { uefi::boot::exit_boot_services(MemoryType::LOADER_DATA) };
    // NOTE: alloc & log are no longer available

    // construct BootInfo
    let bootinfo = BootInfo {
        memory_map: mmap.entries().copied().collect(),
        physical_memory_offset: config.physical_memory_offset,
        system_table,
    };

    // align stack to 8 bytes
    let stacktop = config.kernel_stack_address + config.kernel_stack_size * 0x1000 - 8;

    jump_to_entry(&bootinfo, stacktop);
}
