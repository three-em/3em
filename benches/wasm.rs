#[macro_use]
extern crate three_em;

use std::cell::Cell;
use three_em::runtime::smartweave::ContractInfo;

use criterion::black_box;
use criterion::criterion_group;
use criterion::criterion_main;
use criterion::Criterion;
use deno_core::serde_json;
use deno_core::serde_json::json;
use std::time::Duration;
use three_em::runtime::wasm::WasmRuntime;
use wasmer::Universal;
use wasmer::{
  imports, Function, FunctionType, Instance, MemoryView, Module, Store, Type,
};

fn wasmer_bench(
  instance: Instance,
  state: &mut [u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
  let handle =
    instance
      .exports
      .get_function("handle")?
      .native::<(u32, u32, u32, u32, u32, u32), u32>()?;

  let alloc = instance
    .exports
    .get_function("_alloc")?
    .native::<u32, u32>()?;
  let get_len = instance
    .exports
    .get_function("get_len")?
    .native::<(), u32>()?;
  let ptr = alloc.call(state.len() as u32)?;

  let memory = instance.exports.get_memory("memory")?;
  let mut raw_mem = unsafe { memory.data_unchecked_mut() };
  raw_mem[ptr as usize..ptr as usize + state.len()].swap_with_slice(state);

  let mut info =
    deno_core::serde_json::to_vec(&ContractInfo::default()).unwrap();
  let info_ptr = alloc.call(info.len() as u32)?;

  let mut action =
    deno_core::serde_json::to_vec(&deno_core::serde_json::json!({})).unwrap();
  let action_ptr = alloc.call(action.len() as u32)?;

  raw_mem[info_ptr as usize..info_ptr as usize + info.len()]
    .swap_with_slice(&mut info);

  raw_mem[action_ptr as usize..action_ptr as usize + action.len()]
    .swap_with_slice(&mut action);

  let result_ptr = handle.call(
    ptr,
    state.len() as u32,
    action_ptr,
    action.len() as u32,
    info_ptr,
    info.len() as u32,
  )? as usize;

  let view: MemoryView<u8> = memory.view();
  let result_len = get_len.call()? as usize;

  let result = view[result_ptr..result_ptr + result_len]
    .iter()
    .map(Cell::get)
    .collect();

  Ok(result)
}

static BENCH_CONTRACT1: &[u8] =
  include_bytes!("../wasm_tools/rust/example/contract.wasm");

fn setup_wasmer(store: Store) -> Result<Instance, Box<dyn std::error::Error>> {
  let module = Module::new(&store, BENCH_CONTRACT1)?;
  let read_state =
    FunctionType::new(vec![Type::I32, Type::I32, Type::I32], vec![Type::I32]);
  let read_state_function = Function::new(&store, &read_state, |_args| {
    // TODO: How do I even access memory from here?
    Ok(vec![wasmer::Value::I32(0)])
  });
  let abort =
    FunctionType::new(vec![Type::I32, Type::I32, Type::I32, Type::I32], vec![]);
  let abort = Function::new(&store, &abort, |_args| Ok(vec![]));

  let import_object = imports! {
    "3em" => {
      "smartweave_read_state" => read_state_function,
    },
    "env" => {
      "abort" => abort,
    }
  };

  let instance = Instance::new(&module, &import_object)?;
  Ok(instance)
}

fn wasm_benchmark(c: &mut Criterion) {
  let rt = tokio::runtime::Runtime::new().unwrap();

  let mut group = c.benchmark_group("WASM");

  group.measurement_time(Duration::from_secs(20));
  group.bench_function("wasmer (default)", |b| {
    b.to_async(&rt).iter_with_setup(
      || {
        let mut state = json!({
          "counter": 0,
        });

        let store = Store::default();
        (deno_core::serde_json::to_vec(&state).unwrap(), store)
      },
      |(state_bytes, store)| async {
        let mut state = state_bytes;
        let instance = setup_wasmer(store).unwrap();
        black_box(wasmer_bench(instance, &mut state).unwrap());
      },
    )
  });

  group.bench_function("wasmer (singlepass)", |b| {
    b.to_async(&rt).iter_with_setup(
      || {
        let mut state = json!({
          "counter": 0,
        });
        let compiler = wasmer_compiler_singlepass::Singlepass::new();
        let store = Store::new(&Universal::new(compiler).engine());

        (deno_core::serde_json::to_vec(&state).unwrap(), store)
      },
      |(state_bytes, store)| async {
        let mut state = state_bytes;
        let instance = setup_wasmer(store).unwrap();
        black_box(wasmer_bench(instance, &mut state).unwrap());
      },
    )
  });

  group.bench_function("v8", |b| {
    b.to_async(&rt).iter_with_setup(
      || {
        let state = json!({
          "counter": 0,
        });
        let state_bytes = serde_json::to_vec(&state).unwrap();

        let action = json!({});
        let action_bytes = serde_json::to_vec(&action).unwrap();
        let mut rt =
          WasmRuntime::new(BENCH_CONTRACT1, Default::default()).unwrap();
        (state_bytes, action_bytes, rt)
      },
      |(state_bytes, action_bytes, mut rt)| async move {
        let mut state = state_bytes;
        let mut action = action_bytes;
        black_box(rt.call(&mut state, &mut action).unwrap());
      },
    )
  });

  group.finish();
}

criterion_group!(benches, wasm_benchmark);
criterion_main!(benches);
