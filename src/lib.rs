use deno_core::{error::AnyError, include_js_files, Extension};
use reqwest::{
    header::{HeaderMap, USER_AGENT},
    redirect::Policy,
    Client,
};

mod fetch;

use fetch::op_fetch;
use fetch::op_fetch_send;

#[derive(Clone)]
pub struct Options {
    pub user_agent: String,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            user_agent: "".to_string(),
        }
    }
}

pub fn init(options: Options) -> Extension {
    Extension::builder()
        .js(include_js_files!(prefix "fetch", "javascript/fetch.js", ))
        .ops(vec![op_fetch::decl(), op_fetch_send::decl()])
        .state(move |state| {
            state.put::<Options>(options.clone());
            state.put::<reqwest::Client>(create_http_client(options.user_agent.clone()).unwrap());
            Ok(())
        })
        .build()
}

pub fn create_http_client(user_agent: String) -> Result<Client, AnyError> {
    // todo(tls):
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, user_agent.parse().unwrap());
    let builder = Client::builder()
        .redirect(Policy::none())
        .default_headers(headers);
    // todo(proxy):
    Ok(builder.build().unwrap())
}
