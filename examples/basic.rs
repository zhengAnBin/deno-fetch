use std::rc::Rc;

use deno_core::anyhow::Error;
use deno_core::{FsModuleLoader, JsRuntime, RuntimeOptions};

struct Permissions;

impl deno_web::TimersPermission for Permissions {
    fn allow_hrtime(&mut self) -> bool {
        unreachable!("snapshotting!")
    }

    fn check_unstable(&self, _state: &deno_core::OpState, _api_name: &'static str) {
        unreachable!("snapshotting!")
    }
}

// #[tokio::main]
fn main() -> Result<(), Error> {
    let mut js_runtime = JsRuntime::new(RuntimeOptions {
        module_loader: Some(Rc::new(FsModuleLoader)),
        extensions: vec![
            deno_webidl::init(),
            deno_url::init(),
            deno_web::init::<Permissions>(deno_web::BlobStore::default(), Default::default()),
            fetch::init(Default::default()),
        ],
        ..Default::default()
    });
    let main_module_url = format!("{}/examples/basic.js", env!("CARGO_MANIFEST_DIR"));
    println!("{:?}", &main_module_url);
    let tokio_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let main_module = deno_core::resolve_path(&main_module_url)?;

    let future = async move {
        let mod_id = js_runtime.load_main_module(&main_module, None).await?;
        let result = js_runtime.mod_evaluate(mod_id);
        js_runtime.run_event_loop(false).await?;
        result.await?
    };
    tokio_runtime.block_on(future)
}
