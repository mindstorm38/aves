//! Paged memory allocation.

use core::num::NonZeroUsize;
use core::ptr::NonNull;
use core::mem::size_of;

use bitflags::bitflags;

use crate::asm::{LD_HEAP_START, LD_HEAP_SIZE};


/// Size of an allocated page.
pub const PAGE_SIZE: usize = 4096;

/// The index of the first allocable page. Previous pages are
/// used to store metadata about allocated pages.
/// 
/// Once set, this value should be at least 1, because we will always 
/// have a page at least for metadata itself.
static mut PAGE_START: usize = 0;


bitflags! {
    /// Metadata flags for a single page.
    struct PageFlags: u8 {
        /// Special constant for empty pages.
        const EMPTY     = 0b0000_0000;
        /// The page is already allocated.
        const TAKEN     = 0b0000_0001;
        /// The page is the first of an allocation.
        const FIRST     = 0b0000_0010;
        /// The page is already allocated for metadata usage.
        const METADATA  = 0b1000_0000;
    }
}

/// Represent a metadata for a single page.
#[repr(C)]
struct PageMetadata {
    flags: PageFlags
}

impl PageMetadata {

    #[inline]
    pub fn is_taken(&self) -> bool {
        self.flags.contains(PageFlags::TAKEN)
    }

    #[inline]
    pub fn is_first(&self) -> bool {
        self.flags.contains(PageFlags::FIRST)
    }

    #[inline]
    pub fn is_taken_and_first(&self) -> bool {
        self.flags.contains(PageFlags::TAKEN | PageFlags::FIRST)
    }

    #[inline]
    pub fn is_taken_and_not_first(&self) -> bool {
        self.flags & (PageFlags::TAKEN | PageFlags::FIRST) == PageFlags::TAKEN
    }

}


/// Get a slice of all pages metadata. 
/// 
/// *This function is unsafe, because caller must ensure that 
/// no concurrent access to the metadata is made.*
#[inline(always)]
unsafe fn get_pages() -> &'static mut [PageMetadata] {
    let len = LD_HEAP_SIZE / PAGE_SIZE;
    let ptr = LD_HEAP_START as *mut PageMetadata;
    core::slice::from_raw_parts_mut(ptr, len)
}


/// Initialize the page system.
/// 
/// *This function is unsafe because it must be called before
/// any other page allocation function.*
pub unsafe fn init() {

    let pages = get_pages();

    // Here we compute the number of reserved pages, used only for pages metadata.
    let pages_meta_total_size = pages.len() * size_of::<PageMetadata>();
    PAGE_START = (pages_meta_total_size + PAGE_SIZE - 1) / PAGE_SIZE;

    for (i, page) in pages.iter_mut().enumerate() {
        if i < PAGE_START {
            // The flags for metadata-reserved pages should not change afterward.
            page.flags = PageFlags::TAKEN | PageFlags::METADATA;
        } else {
            page.flags = PageFlags::EMPTY;
        }
    }

}


/// Allocate the given number of pages.
/// The returned pointer is aligned to `PAGE_SIZE` (4096).
/// 
/// *This function is unsafe because caller must ensure
/// that no concurrent call is made to this function or
/// other allocation-related functions.*
pub unsafe fn alloc(pages_count: NonZeroUsize) -> Result<NonNull<u8>, AllocError> {

    let pages_count = pages_count.get();
    let pages = get_pages();

    let mut first_page = 0;
    let mut found = false;
    let mut pages_needed = 0;

    for (i, page) in pages.iter().enumerate().skip(PAGE_START) {

        if !page.is_taken() {
            
            if pages_needed == 0 {
                // If pages needed equals 0, this is a sentinel that means
                // we are starting a new free-pages sequence.
                pages_needed = pages_count - 1;
                first_page = i;
            } else {
                // Here we decrement pages needed and if we reach 0, this
                // means that we found anough pages for our allocation.
                pages_needed -= 1;
            }

            // This branch is in common because in case of pages count 1,
            // we directly reach this condition.
            if pages_needed == 0 {
                found = true;
                break;
            }

        } else {
            // If the page is taken, just reset the counter and start searching again.
            pages_needed = 0;
        }

    }

    if found {

        for page in &mut pages[(first_page + 1)..(first_page + pages_count)] {
            page.flags = PageFlags::TAKEN;
        }

        pages[first_page].flags = PageFlags::TAKEN | PageFlags::FIRST;

        Ok(NonNull::new_unchecked(LD_HEAP_START.add(first_page * PAGE_SIZE) as _))

    } else {
        Err(AllocError)
    }

}


/// Allocate the given number of pages and set all the data to 0.
/// The returned pointer is aligned to `PAGE_SIZE` (4096).
/// 
/// *This function is unsafe because caller must ensure
/// that no concurrent call is made to this function or
/// other allocation-related functions.*
pub unsafe fn alloc_zeroed(pages_count: NonZeroUsize) -> Result<NonNull<u8>, AllocError> {
    alloc(pages_count).map(|nnptr| {
        // Since the returned data is a multiple of PAGE_SIZE (4096),
        // it's safe and valid to fill it 8 by 8 bytes.
        let mut ptr = nnptr.as_ptr() as *mut u64;
        let len = pages_count.get() * PAGE_SIZE / 8;
        for _ in 0..len {
            *ptr = 0;
            ptr = ptr.add(1);
        }
        nnptr
    })
}


/// Deallocate previsouly allocated pages with [`alloc_raw`]. 
/// The given address is aligned to [`PAGE_SIZE`] anyway.
/// 
/// *This function is unsafe because caller must ensure
/// that no concurrent call is made to this function or
/// other allocation-related functions.*
pub unsafe fn dealloc(page: NonNull<u8>) -> Result<(), DeallocError> {

    // Compute the page index (this inerently align the pointer).
    let page_index = page.as_ptr().offset_from(LD_HEAP_START) as usize / PAGE_SIZE;
    
    let pages = get_pages();
    
    if let Some(pages) = pages.get_mut(page_index..) {

        if !pages[0].is_taken_and_first() {
            Err(DeallocError::InvalidPointer)
        } else {

            pages[0].flags = PageFlags::EMPTY;

            // Here we take all subsequent pages that a both taken 
            // and not a first one. Encountering a first page would
            // mean that we are on another allocation.
            pages[1..]
                .iter_mut()
                .take_while(|page| page.is_taken_and_not_first())
                .for_each(|page| page.flags = PageFlags::EMPTY);

            Ok(())

        }

    } else {
        Err(DeallocError::OutOfRangePointer)
    }
    
}


/// A marker error when an allocation fails.
#[derive(Debug, Clone, Copy)]
pub struct AllocError;


/// Describe an error that can happen if some preconditions 
/// are not met when deallocating.
#[derive(Debug, Clone, Copy)]
pub enum DeallocError {
    /// The given pointer points to a free or non-first memory page.
    InvalidPointer,
    /// The given pointer is out of the valid memory pages range.
    OutOfRangePointer,
}


/// Compute an information structure.
pub unsafe fn info() -> PageMemoryInfo {

    let pages = get_pages();
    let metadata_usage_split_addr = LD_HEAP_START.add(PAGE_START * PAGE_SIZE).addr();
    let pages_count = pages.len();

    let mut info = PageMemoryInfo {
        metadata_pages_start: LD_HEAP_START.addr(),
        metadata_pages_end: metadata_usage_split_addr,
        usable_pages_start: metadata_usage_split_addr,
        usable_pages_end: LD_HEAP_START.add(pages_count * PAGE_SIZE).addr(),
        metadata_pages_count: PAGE_START,
        usable_pages_count: pages_count - PAGE_START,
        total_pages_count: pages_count,
        allocated_pages_count: 0,
        free_pages_count: 0,
        allocations_count: 0,
    };

    for page in &pages[PAGE_START..] {
        if page.is_taken() {
            if page.is_first() {
                info.allocations_count += 1;
            }
            info.allocated_pages_count += 1;
        } else {
            info.free_pages_count += 1;
        }
    }

    info

}


#[derive(Debug)]
#[repr(C)]
pub struct PageMemoryInfo {
    /// Address of the first metadata page.
    pub metadata_pages_start: usize,
    /// Last address of the last metadata page.
    pub metadata_pages_end: usize,
    /// Address of the first usable page.
    pub usable_pages_start: usize,
    /// Last address of the last usable page.
    pub usable_pages_end: usize,
    /// Number of pages used for metadata.
    pub metadata_pages_count: usize,
    /// Number of usable pages.
    /// This must be equal to `allocated_pages_count + free_pages_count`.
    pub usable_pages_count: usize,
    /// Number of pages, counting both metadata and usable ones.
    pub total_pages_count: usize,
    /// Number of allocated pages.
    pub allocated_pages_count: usize,
    /// Number of free pages.
    pub free_pages_count: usize,
    /// Number of allocations.
    pub allocations_count: usize,
}


/*bitflags! {
    /// Metadata flags for a single page.
    pub struct EntryFlags: u8 {

        /// Special constant for empty entry.
        const EMPTY     = 0b0000_0000;
        /// The entry is valid.
        const VALID     = 0b0000_0001;
        /// The entry maps to a readable memory.
        const READ      = 0b0000_0010;
        /// The entry maps to a writable memory.
        const WRITE     = 0b0000_0100;
        /// The entry maps to an executable memory.
        const EXECUTE   = 0b0000_1000;
        /// The page is accessible to user mode.
        const USER      = 0b0001_0000;
        /// This mapping exists in all address spaces.
        const GLOBAL    = 0b0010_0000;
        /// Set to 1 if the page has been read/written or fetched since last bit clear.
        const ACCESSED  = 0b0100_0000;
        /// The page has been written since last bit clear.
        const DIRTY     = 0b1000_0000;

        const READ_WRITE = Self::READ.bits | Self::WRITE.bits;
        const READ_EXECUTE = Self::READ.bits | Self::EXECUTE.bits;
        const READ_WRITE_EXECUTE = Self::READ.bits | Self::WRITE.bits | Self::EXECUTE.bits;

    }
}


#[repr(C)]
pub union Entry {
    raw: u64,
    flags: EntryFlags,
}

impl Entry {

    #[inline]
    pub fn get_flags(&self) -> EntryFlags {
        unsafe { self.flags }
    }
    
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.get_flags().contains(EntryFlags::VALID)
    }

    /// Return true if the entry maps to a physical address.
    #[inline]
    pub fn is_leaf(&self) -> bool {
        self.get_flags() & EntryFlags::READ_WRITE_EXECUTE != EntryFlags::EMPTY
    }

    #[inline]
    pub fn set_leaf(&mut self, paddr: usize, flags: EntryFlags) {
        // Physical address are limited to 44 bits and we remove the 12 least bits.
        self.raw = ((paddr & 0xFFF_FFFF_F000) >> 2) as u64;
        self.flags = flags | EntryFlags::VALID | EntryFlags::DIRTY | EntryFlags::ACCESSED;
    }

    /// Return true if the entry points to the next level of page table.
    #[inline]
    pub fn is_branch(&self) -> bool {
        !self.is_leaf()
    }

    /// Set this entry has a branch to a next level of page table.
    #[inline]
    pub fn set_branch(&mut self, table: NonNull<Table>) {
        // >> 2 because >> 12 << 10
        self.raw = ((table.addr().get() & !0x3FF) >> 2) as u64;
        self.flags = EntryFlags::VALID;
    }

    #[inline]
    pub fn get_branch(&self) -> Option<NonNull<Table>> {
        unsafe {
            if self.flags & (EntryFlags::READ_WRITE_EXECUTE | EntryFlags::VALID) == EntryFlags::VALID {
                // >> 2 because >> 12 << 10
                let ppn = (self.raw & 0x003F_FFFF_FFFF_FC00) << 2;
                NonNull::new(ppn as *mut Table)
            } else {
                None
            }
        }
    }

}


/// Represent a RAII paging table used by the MMU.
pub struct Table {
    entries: NonNull<TableEntries>
}

/// Type alias for table entries.
pub type TableEntries = [Entry; 512];

impl Table {

    /// Allocate a new paging table.
    /// 
    /// *This function is unsafe because caller must ensure
    /// that no concurrent call is made to this function or
    /// other allocation-related functions.*
    pub unsafe fn new() -> Result<Self, AllocError> {
        // We allocate one page, it's enough for the whole table structure.
        debug_assert_eq!(size_of::<Table>(), PAGE_SIZE);
        alloc_zeroed(NonZeroUsize::new_unchecked(1)).map(|p| Table { 
            entries: p.cast() 
        })
    }

    /// Map a virtual address to a physical address for this table.
    /// This function might fail if a sub-table allocation fails.
    /// 
    /// *This function is unsafe because caller must ensure
    /// that no concurrent call is made to this function or
    /// other allocation-related functions.*
    pub unsafe fn map(&mut self, vaddr: usize, paddr: usize, flags: EntryFlags) -> Result<(), AllocError> {

        let vpn = [
            (vaddr >> 12) & 0x1FF,
            (vaddr >> 21) & 0x1FF,
            (vaddr >> 30) & 0x1FF,
        ];

        let mut entry = &mut self.entries[vpn[2]];

        for i in (0..2).rev() {

            let level_table;
            if let Some(table) = entry.get_branch() {
                level_table = table;
            } else {
                level_table = Self::new()?;
                entry.set_branch(level_table);
            };

            entry = &mut (*level_table.as_ptr()).entries[vpn[i]];

        }

        entry.set_leaf(paddr, flags);
        Ok(())

    }

    /// Unmap all branches tables, this doesn't free the 'self' one.
    pub unsafe fn unmap(&mut self) {
        self.unmap_internal(true);
    }

    unsafe fn unmap_internal(&mut self, empty: bool) {
        for entry in &mut self.entries {
            if let Some(table) = entry.get_branch() {
                // Next level tables don't need to empty entries.
                (*table.as_ptr()).unmap_internal(false);
                let _ = dealloc(table.cast());
            }
            if empty {
                entry.raw = 0;
            }
        }
    }

}
*/