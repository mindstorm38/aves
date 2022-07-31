#![no_main]
#![no_std]
#![feature(allocator_api)]
#![feature(strict_provenance)]
#![feature(panic_info_message)]

pub mod macros;
pub mod asm;
pub mod uart;
pub mod page;
pub mod lock;


/// The main entry point of the kernel, called from `boot.asm`.
#[no_mangle]
extern "C" fn kmain() {
    
    unsafe { uart::init(); }
    println!("== Starting Aves 0.1.0");
    println!(" = UART initialized");

    unsafe { page::init(); }
    let info = unsafe { page::info() };
    println!(" = Page allocator initialized");
    println!("   Meta: 0x{:08X} -> 0x{:08X} ({})", info.metadata_pages_start, info.metadata_pages_end, info.metadata_pages_count);
    println!("   Usbl: 0x{:08X} -> 0x{:08X} ({})", info.usable_pages_start, info.usable_pages_end, info.usable_pages_count);

    loop {
        if let Some(c) = unsafe { uart::get() } {
            match c {
                8 => print!("\x08 \x08"),
                10 | 13 => println!(),
                _ => print!("{}", c as char),
            }
        }
    }

}


#[no_mangle]
extern "C" fn trap_vector(mcause: u64, mtval: u64) {
    /*if mcause & 0x8000_0000_0000_0007 == 0x8000_0000_0000_0007 {
        println!("machine timer interrupt");
    }*/
    // println!("trap: mcause: {:08X}, mtval: {:08X}", mcause, mtval);
}


/// Panic handler will only abort.
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("== A hart panicked...");
    if let Some(loc) = info.location() {
        println!(" = At: {}", loc);
    }
    if let Some(msg) = info.message() {
        println!(" = Message: {}", msg);
    }
    unsafe { asm::asm_abort() }
}
