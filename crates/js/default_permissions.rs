use deno_core::OpState;
use std::path::Path;

pub struct Permissions;

impl deno_web::TimersPermission for Permissions {
  fn allow_hrtime(&mut self) -> bool {
    true
  }
  fn check_unstable(&self, state: &OpState, api_name: &'static str) {}
}

impl deno_fetch::FetchPermissions for Permissions {
  fn check_net_url(
    &mut self,
    _url: &deno_core::url::Url,
  ) -> Result<(), deno_core::error::AnyError> {
    Ok(())
  }

  fn check_read(
    &mut self,
    _p: &Path,
  ) -> Result<(), deno_core::error::AnyError> {
    Ok(())
  }
}
