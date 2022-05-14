use deno_core::error::type_error;
use deno_core::error::AnyError;
use deno_core::futures::FutureExt;
use deno_core::ModuleLoader;
use deno_core::ModuleSpecifier;
use deno_core::ModuleType;

use std::pin::Pin;

pub struct EmbeddedModuleLoader(pub String, pub String);

impl ModuleLoader for EmbeddedModuleLoader {
  fn resolve(
    &self,
    specifier: &str,
    _referrer: &str,
    _is_main: bool,
  ) -> Result<ModuleSpecifier, AnyError> {
    if let Ok(module_specifier) = deno_core::resolve_url(specifier) {
      if specifier == self.1 {
        return Ok(module_specifier);
      }
    }

    Err(type_error("Module loading prohibited."))
  }

  fn load(
    &self,
    module_specifier: &ModuleSpecifier,
    _maybe_referrer: Option<ModuleSpecifier>,
    _is_dynamic: bool,
  ) -> Pin<Box<deno_core::ModuleSourceFuture>> {
    let module_specifier = module_specifier.clone();

    let code = self.0.to_string();
    async move {
      let specifier = module_specifier.to_string();

      Ok(deno_core::ModuleSource {
        code,
        module_url_specified: specifier.clone(),
        module_url_found: specifier,
        module_type: ModuleType::JavaScript,
      })
    }
    .boxed_local()
  }
}
