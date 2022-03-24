//! Allocator APIs exposed to the host.
use ::std::alloc::alloc;
use ::std::alloc::dealloc;
use ::std::alloc::Layout;
use ::std::mem::align_of;

#[no_mangle]
pub unsafe fn contract_alloc(len: usize) -> *mut u8 {
  let align = align_of::<usize>();
  let layout = Layout::from_size_align_unchecked(len, align);
  alloc(layout)
}

#[no_mangle]
pub unsafe fn contract_dealloc(ptr: *mut u8, size: usize) {
  let align = align_of::<usize>();
  let layout = Layout::from_size_align_unchecked(size, align);
  dealloc(ptr, layout);
}
