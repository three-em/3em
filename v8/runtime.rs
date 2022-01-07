use deno_core::serde_v8;
use deno_core::JsRuntime;
use deno_core::RuntimeOptions;
use serde::Serialize;
use traits::Result;

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum HeapLimitState {
  /// Ok, the heap limit is not exceeded.
  Ok,
  /// The heap limit is exceeded.
  Exceeded(usize),
}

impl Default for HeapLimitState {
  fn default() -> Self {
    HeapLimitState::Ok
  }
}

pub struct V8RuntimeOptions<T: Serialize> {
  pub source: String,
  pub state: T,
}

pub struct V8Runtime {
  runtime: JsRuntime,
  state: Rc<RefCell<HeapLimitState>>,
  handler: v8::Global<v8::Function>,
  contract_state: v8::Global<v8::Value>,
}

fn v8_prepare() {
  let flags = concat!(
    "--predictable",
    " --predictable-gc-schedule",
    " --hash-seed=42",
    " --random-seed=42",
  );
  v8::V8::set_flags_from_string(flags);

  // Make's Math.random() and V8 hash seeds, address space layout repr deterministic.
  v8::V8::set_entropy_source(|buf| {
    for c in buf {
      *c = 42;
    }
    true
  });
}

async fn create_v8_runtime() -> Result<(Rc<RefCell<HeapLimitState>>, JsRuntime)> {
  let params = v8::CreateParams::default().heap_limits(0, 5 << 20);
  let mut rt = JsRuntime::new(RuntimeOptions {
    extensions: vec![
      deno_webidl::init(),
      deno_url::init(),
      deno_web::init(deno_web::BlobStore::default(), None),
      deno_crypto::init(Some(0)),
    ],
    // startup_snapshot: Some(snapshot::snapshot()),
    create_params: Some(params),
    ..Default::default()
  });

  let isolate = rt.v8_isolate();

  let handle = isolate.thread_safe_handle();
  let state = Rc::new(RefCell::new(HeapLimitState::default()));

  let state_clone = state.clone();
  rt.add_near_heap_limit_callback(move |curr, _| {
    let terminated = handle.terminate_execution();
    assert!(terminated);

    *state_clone.borrow_mut() = HeapLimitState::Exceeded(curr);
    (curr + 5) << 20
  });

  rt.sync_ops_cache();

  rt.run_event_loop(false).await?;

  Ok((state, rt))
}

fn execute_script(
  context_scope: &mut v8::ContextScope<v8::HandleScope>,
  script: v8::Local<v8::String>,
) {
  let scope = &mut v8::HandleScope::new(context_scope);
  let try_catch = &mut v8::TryCatch::new(scope);

  let script = v8::Script::compile(try_catch, script, None)
    .expect("failed to compile script");

  if script.run(try_catch).is_none() {
    let exception_string = try_catch
      .stack_trace()
      .or_else(|| try_catch.exception())
      .map(|value| value.to_rust_string_lossy(try_catch))
      .unwrap_or_else(|| "no stack trace".into());

    panic!("{}", exception_string);
  }
}

impl V8Runtime {
  pub async fn new<T>(options: V8RuntimeOptions<T>) -> Result<Self>
  where
    T: Serialize + 'static,
  {
    // TODO: SetOnce
    v8_prepare();
    let (state, mut runtime) = create_v8_runtime().await?;
    let (contract_state, handler) = {
      let mut scope = runtime.handle_scope();

      let source = v8::String::new(&mut scope, &options.source).unwrap();

      let context = v8::Context::new(&mut scope);
      let mut context_scope = v8::ContextScope::new(&mut scope, context);

      execute_script(&mut context_scope, source);

      let handle_str = v8::String::new(&mut context_scope, "handle").unwrap();
      let handler = context
        .global(&mut context_scope)
        .get(&mut context_scope, handle_str.into())
        .expect("missing function handle");
      let handler = v8::Local::<v8::Function>::try_from(handler)?;
      let local = serde_v8::to_v8(&mut context_scope, options.state)?;

      let contract_state = v8::Global::new(&mut context_scope, local);
      let handler = v8::Global::new(&mut context_scope, handler);

      (contract_state, handler)
    };

    Ok(Self {
      runtime,
      state,
      handler,
      contract_state,
    })
  }

  pub async fn call<R>(&mut self, action: R) -> Result<()>
  where
    R: Serialize + 'static,
  {
    let global = self.call_handler(action)?;
    // Run the event loop.
    let global = self.runtime.resolve_value(global).await?;

    let mut scope = self.runtime.handle_scope();

    let state = v8::Local::new(&mut scope, global)
      .to_object(&mut scope)
      .unwrap();
    let state_key = v8::String::new(&mut scope, "state").unwrap().into();

    let state_obj = state.get(&mut scope, state_key).unwrap();
    if let Some(state) = state_obj.to_object(&mut scope) {
      // Update the contract state.
      self.contract_state = v8::Global::new(&mut scope, state_obj);
    }

    Ok(())
  }

  fn call_handler<R>(&mut self, action: R) -> Result<v8::Global<v8::Value>>
  where
    R: Serialize + 'static,
  {
    let mut scope = self.runtime.handle_scope();

    let action = serde_v8::to_v8(&mut scope, action).unwrap();
    let handler = self.handler.open(&mut scope);

    let state =
      v8::Local::<v8::Value>::new(&mut scope, self.contract_state.clone());
    let undefined = v8::undefined(&mut scope);
    let local = handler
      .call(&mut scope, undefined.into(), &[state, action])
      .unwrap();

    Ok(v8::Global::new(&mut scope, local))
  }

  pub fn get_state(&self) -> Result<()> {
    unimplemented!()
  }
}
