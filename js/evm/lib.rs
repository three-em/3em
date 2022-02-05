use evm::Instruction;
use evm::Machine;
use evm::U256;
use std::alloc::Layout;

fn dummy_cost_fn(_: &Instruction) -> U256 {
  U256::zero()
}

#[no_mangle]
pub extern "C" fn machine_new() -> *mut Machine<'static> {
  Box::into_raw(Box::new(Machine::new(dummy_cost_fn)))
}

#[no_mangle]
pub extern "C" fn machine_new_with_data(
  data: *mut u8,
  data_len: usize,
) -> *mut Machine<'static> {
  let data = unsafe { Vec::from_raw_parts(data, data_len, data_len) };
  Box::into_raw(Box::new(Machine::new_with_data(dummy_cost_fn, data)))
}

#[no_mangle]
pub extern "C" fn machine_free(machine: *mut Machine) {
  unsafe {
    Box::from_raw(machine);
  }
}

#[no_mangle]
pub extern "C" fn machine_result(machine: *mut Machine) -> *const u8 {
  let machine = unsafe { Box::from_raw(machine) };
  let ptr = machine.result.as_ptr();
  Box::leak(machine);
  ptr
}

#[no_mangle]
pub extern "C" fn machine_result_len(machine: *mut Machine) -> usize {
  let machine = unsafe { Box::from_raw(machine) };
  let length = machine.result.len();
  Box::leak(machine);
  length
}

#[no_mangle]
pub extern "C" fn machine_execute(
  machine: *mut Machine,
  ptr: *const u8,
  length: usize,
) -> *mut Machine {
  let mut machine = unsafe { Box::from_raw(machine) };
  let bytecode = unsafe { std::slice::from_raw_parts(ptr, length) };

  let status = machine.execute(bytecode, Default::default());
  if status != evm::ExecutionState::Ok {
    panic!("Execution failed");
  }
  Box::into_raw(machine)
}

#[no_mangle]
pub unsafe fn alloc(len: usize) -> *mut u8 {
  let align = std::mem::align_of::<usize>();
  let layout = Layout::from_size_align_unchecked(len, align);
  std::alloc::alloc(layout)
}
