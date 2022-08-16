//! Process-related structures and functions.

pub mod builtin;

use core::marker::PhantomData;
use core::num::NonZeroUsize;
use core::ptr::NonNull;
use core::mem::size_of;

use crate::memory::page::{PAGE_SIZE, alloc, dealloc};
use crate::println;


/// Maximum number of processes a memory page can hold.
/// This is used to reallocate a new page that will be linked
/// at the end of the previous page.
const PROCESS_COUNT_PER_PAGE: usize = PAGE_SIZE / size_of::<ProcessEntry>() - 1;

/// Maximum length of the process' name.
const PROCESS_NAME_MAX_LEN: usize = 128;

/// A pointer to the first process page.
static mut FIRST_PROCESS_PAGE: NonNull<ProcessEntry> = NonNull::dangling();

/// A pointer to the last process page.
static mut LAST_PROCESS_PAGE: NonNull<ProcessEntry> = NonNull::dangling();

/// Current number of process pages.
static mut PROCESS_PAGE_COUNT: usize = 0;

/// Current number of allocated processes (dead or not).
static mut PROCESS_COUNT: usize = 0;

static mut RUNNING_PROCESS: Option<NonNull<Process>> = None;


/// Type alias for a Process ID, returned upon process spawn.
pub type Pid = usize;


/// Size of: 289
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Process {
    /// Process ID of the process. [offset 0]
    pid: Pid,
    /// Process ID of the parent process. [offset 8]
    parent_pid: Pid,
    /// Start of the stack. [offset 16]
    stack_start: usize,
    /// End of the stack (biggest address, where sp starts). [offset 24]
    stack_end: usize,
    /// Saved context. [offset 32]
    context: Context,
    /// Length of the name of the process. [offset 152]
    name_len: usize,
    /// Name of the process. Guaranteed to be UTF-8 until length is reached. [offset 160]
    name: [u8; PROCESS_NAME_MAX_LEN],
    /// State of the process, if dead, the entry should be ignored. [offset 288]
    state: ProcessState,
}


/// Size of: 120
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Context {
    /// Program counter.
    pc: usize,
    /// Stack pointer.
    sp: usize,
    /// Unused
    _unused: usize,
    /// Saved registers (s0-s11).
    sx: [usize; 12],
}

impl Process {

    /// Get the name of the process.
    #[inline]
    fn name(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self.name[..self.name_len]) }
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(unused)]
enum ProcessState {
    /// An invalid process is a marker for unused process entries.
    Invalid     = 0x0,
    /// The process has just been spawned, not yet run.
    Spawned     = 0x1,
    /// The process is waiting to be resumed.
    Waiting     = 0x2,
    /// The process is currently running.
    Running     = 0x3,
    /// A process that returned from its entry point.
    Dead        = 0x4,
}

#[repr(C)]
#[derive(Clone, Copy)]
union ProcessEntry {
    /// Interpret the entry as a process.
    process: Process,
    /// Interpret the entry as the next process page.
    next_page: NonNull<ProcessEntry>,
}



/// Initialize the process manager.
/// 
/// *This function is unsafe because you must ensure that
/// it is called once and after the initialization of the
/// page memory allocator.*
pub unsafe fn init() {
    FIRST_PROCESS_PAGE = alloc(NonZeroUsize::new_unchecked(1)).unwrap().cast();
    LAST_PROCESS_PAGE = FIRST_PROCESS_PAGE;
    PROCESS_PAGE_COUNT = 1;
}


/// Spawn a new process.
pub fn spawn(entry_point: extern "C" fn(), name: &str) -> Pid {

    debug_assert!(name.len() <= PROCESS_NAME_MAX_LEN);

    unsafe {

        if PROCESS_COUNT > 0 && PROCESS_COUNT % PROCESS_COUNT_PER_PAGE == 0 {

            let new_process_page = alloc(NonZeroUsize::new_unchecked(1)).unwrap().cast();

            let next_page_ptr = LAST_PROCESS_PAGE.as_ptr().add(PROCESS_COUNT_PER_PAGE);
            let next_page = &mut (*next_page_ptr).next_page;

            *next_page = new_process_page;

            LAST_PROCESS_PAGE = new_process_page;
            PROCESS_PAGE_COUNT += 1;

        }

        let stack_ptr = alloc(NonZeroUsize::new_unchecked(1)).unwrap();

        // In the future, we might reuse old processes, but not for now.
        let pid = PROCESS_COUNT;
        
        // Index of the process in the last page.
        let process_index = pid - (PROCESS_PAGE_COUNT - 1) * PROCESS_COUNT_PER_PAGE;
        
        let process_ptr = LAST_PROCESS_PAGE.as_ptr().add(process_index);
        let process = &mut (*process_ptr).process;

        process.state = ProcessState::Spawned;
        process.pid = pid;
        process.parent_pid = 0;
        process.stack_start = stack_ptr.as_ptr().addr();
        process.stack_end = stack_ptr.as_ptr().add(PAGE_SIZE).addr();

        process.context.pc = (entry_point as *mut u8).addr();
        process.context.sp = process.stack_end;
        process.context.sx.fill(0);

        process.name_len = name.len();
        process.name[..name.len()].clone_from_slice(name.as_bytes());

        PROCESS_COUNT += 1;
        pid

    }

}


extern "C" {
    
    /// This function is defined in `proc.asm` and will switch from
    /// one process to another. The context will be saved in the 
    /// `from` process and restored for `to`.
    fn asm_process_switch(to: *mut Process, exit_fn: extern "C" fn() -> !, from: *mut Process);
    
    /// This function is defined in `proc.asm` and will switch to a
    /// given process. The context of the target process will be restored.
    fn asm_process_switch_noreturn(to: *mut Process, exit_fn: extern "C" fn() -> !);

}


/// Put the calling process in waiting state. This will switch to 
/// another waiting process or do nothing if no process is running.
/// 
/// This function is guaranteed to return at some point when another 
/// process will decides to wait. The registers that are marked as
/// `preserved` in the [`RISC-V programmer's manual`] are saved.
/// 
/// [`RISC-V programmer's manual`]: https://github.com/riscv-non-isa/riscv-asm-manual/blob/master/riscv-asm.md
pub fn wait() {
    unsafe {
        if let Some(process) = RUNNING_PROCESS {
            let current_process = &mut *process.as_ptr();
            if let Some(next_process) = get_next_process(current_process.pid) {
                current_process.state = ProcessState::Waiting;
                RUNNING_PROCESS = Some(next_process.into());
                asm_process_switch(next_process, exit, current_process);
            }
        }
    }
}


/// Exit from the current process and resume other awaiting processes.
pub extern "C" fn exit() -> ! {
    unsafe {

        if let Some(process) = RUNNING_PROCESS {

            let current_process = &mut *process.as_ptr();
            current_process.state = ProcessState::Dead;

            // Free the stack page.
            dealloc(NonNull::new_unchecked(current_process.stack_start as *mut u8)).unwrap();

            current_process.context.pc = 0;
            current_process.context.sp = 0;

            if let Some(next_process) = get_next_process(current_process.pid) {
                RUNNING_PROCESS = Some(next_process.into());
                asm_process_switch_noreturn(next_process, exit);
                // We should never get here even if the method has not explicitly
                // the '!' never return type. Because the process is marked 'Dead',
                // we should never get back here.
            }

        }

        // If no process is running or if there is no next process,
        // just abort the hart to avoid returning from the function.
        println!("== Last process exited on hart #{}, aborting...", crate::cpu::mhardid::get());
        crate::asm::asm_abort();

    }
}


/// Get the PID of the current process.
#[inline]
pub fn pid() -> Pid {
    unsafe { (*RUNNING_PROCESS.unwrap().as_ptr()).pid }
}


/// Start the schedule process, *this should be called once when starting
/// the kernel*.
pub unsafe fn start_schedule() -> ! {
    debug_assert!(RUNNING_PROCESS.is_none());
    if let Some(process) = iter().next() {
        RUNNING_PROCESS = Some(process.into());
        asm_process_switch_noreturn(process, exit);
    }
    println!("== No process to start scheduling on hart #{}, aborting...", crate::cpu::mhardid::get());
    crate::asm::asm_abort();
}


/// Internal function to get the next process to run regarding the
/// current one.
unsafe fn get_next_process<'a>(current_pid: Pid) -> Option<&'a mut Process> {

    let mut first_process = None;
    let mut passed_current = false;

    for process in iter() {
        if process.pid == current_pid {
            passed_current = true;
        } else if let ProcessState::Spawned | ProcessState::Waiting = process.state {
            if passed_current {
                return Some(process);
            } else if first_process.is_none() {
                first_process = Some(process);
            }
        }
    }

    first_process

}


/// Internal function to get a process from its PID.
unsafe fn by_pid(pid: Pid) -> *mut Process {
    let process_idx = pid % PROCESS_COUNT_PER_PAGE;
    let mut page_idx = pid / PROCESS_COUNT_PER_PAGE;
    let mut process_page = FIRST_PROCESS_PAGE.as_ptr();
    while page_idx > 0 {
        process_page = (*process_page.add(PROCESS_COUNT_PER_PAGE)).next_page.as_ptr();
        page_idx -= 1;
    }
    &mut (*process_page.add(process_idx)).process
}


/// Internal function to iterate over running processes.
/// This function should not be 
unsafe fn iter<'a>() -> ProcessIter<'a> {
    ProcessIter {
        page: FIRST_PROCESS_PAGE.as_ptr(),
        index: 0,
        _phantom: PhantomData
    }
}

struct ProcessIter<'a> {
    page: *mut ProcessEntry,
    index: usize,
    _phantom: PhantomData<&'a mut Process>,
}

impl<'a> Iterator for ProcessIter<'a> {
    type Item = &'a mut Process;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            while self.index < PROCESS_COUNT {
                if self.index > 0 && self.index % PROCESS_COUNT_PER_PAGE == 0 {
                    self.page = (*self.page).next_page.as_ptr();
                }
                let process_ptr = &mut (*self.page).process;
                self.index += 1;
                self.page = self.page.add(1);
                if process_ptr.state != ProcessState::Invalid {
                    return Some(process_ptr);
                }
            }
            None
        }
    }
}
