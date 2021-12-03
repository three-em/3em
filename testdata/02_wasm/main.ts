import { JSON, JSONEncoder } from "assemblyscript-json";

export function _alloc(size: usize): usize {
  return heap.alloc(size);
}

let LEN: usize = 0;
export function get_len(): usize {
  return LEN;
}

function read_buf(ptr: usize, size: usize): Uint8Array {
  let buf = new Uint8Array(size);
  for (let i = 0 as usize; i < size; i++) {
    buf[i] += load<u8>(ptr + i);
  }
  return buf;
}

export function handle(
  state_ptr: usize,
  state_size: usize,
  action_ptr: usize,
  action_size: usize,
): usize {
  const state = read_buf(state_ptr, state_size);
  const _action = read_buf(action_ptr, action_size);

  let stateObj: JSON.Obj =
    <JSON.Obj> (JSON.parse(String.UTF8.decode(state.buffer)));

  const counter: JSON.Integer | null = stateObj.getInteger("counter");

  if (counter != null) {
    let encoder = new JSONEncoder();
    encoder.pushObject(null);
    encoder.setInteger("counter", (counter.valueOf() + 1) as i64);
    encoder.popObject();

    let result = encoder.serialize();
    LEN = result.byteLength;

    return result.dataStart;
  } else {
    return 0;
  }
}
