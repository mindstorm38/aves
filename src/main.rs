//! The main entry point for the kernel.
//! 
//! Functions called from assembly code are defined in this main module.
//! These extern functions should be prefixed with "k", like `kmain` and 
//! `ktrap`.

#![no_main]
#![no_std]
#![feature(allocator_api)]
#![feature(strict_provenance)]
#![feature(panic_info_message)]
#![feature(concat_idents)]


pub mod asm;
pub mod cpu;

pub mod memory;
pub mod interrupt;
pub mod process;

pub mod filesystem;

pub mod uart;
pub mod trap;

pub mod lock;

pub mod macros;

pub mod driver;

pub mod conf;

// Internal for now, used to prototype the API.
mod api;



/// The main entry point of the kernel, called from `boot.asm`.
/// The kernel will run only on the hart #0.
#[no_mangle]
extern "C" fn kmain() {
    
    unsafe { uart::init(); }
    println!("== Starting Aves 0.1.0");
    println!("== UART initialized");
    println!("== On hart #{}", cpu::mhardid::get());

    if let Some(isa) = cpu::misa::get() {
        println!("== Extensions: {:?}", isa.extensions);
    }

    unsafe { memory::page::init(); }
    let info = unsafe { memory::page::info() };
    println!("== Page allocator initialized");
    println!(" = Meta: 0x{:08X} -> 0x{:08X} ({})", info.metadata_pages_start, info.metadata_pages_end, info.metadata_pages_count);
    println!(" = Usbl: 0x{:08X} -> 0x{:08X} ({})", info.usable_pages_start, info.usable_pages_end, info.usable_pages_count);

    unsafe {
        interrupt::plic::set_threshold(0);
        interrupt::plic::enable(10);
        interrupt::plic::set_priority(10, 1);
    }
    println!("== PLIC Initialized");

    unsafe { conf::register_drivers(); }
    
    for driver in driver::iter() {
        driver.load();
    }



    unsafe { trap::init_hart_zero(); }
    println!("== Interrupt trap initialized");

    unsafe { process::init(); }
    println!("== Process manager initialized");

    process::spawn(process::builtin::init, "[init]");

    println!("== Start scheduling processes");
    unsafe { process::start_schedule(); }

}


/// Called from `trap.asm` when a fatal exception was trapped.
/// 
#[no_mangle]
extern "C" fn kpanic(code: usize) -> ! {
    
    fn get_code(code: usize) -> Option<&'static str> {
        Some(match code {
            0 => "Instruction address misaligned",
            1 => "Instruction access fault",
            2 => "Illegal instruction",
            3 => "Breakpoint",
            4 => "Load address misaligned",
            5 => "Load access fault",
            6 => "Store/AMO address misaligned",
            7 => "Store/AMO access fault",
            12 => "Instruction page fault",
            13 => "Load page fault",
            15 => "Store/AMO page fault",
            _ => return None
        })
    }

    println!("== The hart #{} encountered a fatal exception...", cpu::mhardid::get());
    
    if let Some(code_name) = get_code(code) {
        println!(" = Code: {}", code_name);
    } else {
        println!(" = Unknown code: {:02X}", code);
    }

    unsafe { asm::asm_abort() }

}


/// Panic handler will only abort.
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("== The hart #{} panicked...", cpu::mhardid::get());
    if let Some(loc) = info.location() {
        println!(" = At: {}", loc);
    }
    if let Some(msg) = info.message() {
        println!(" = Message: {}", msg);
    }
    unsafe { asm::asm_abort() }
}
