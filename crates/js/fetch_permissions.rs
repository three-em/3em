use std::path::Path;

pub struct Permissions;

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