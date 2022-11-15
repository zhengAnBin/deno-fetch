use std::borrow::Cow;
use std::{future::Future, pin::Pin, rc::Rc};

use deno_core::futures::Stream;
use deno_core::futures::StreamExt;
use deno_core::AsyncResult;
use deno_core::CancelTryFuture;
use deno_core::RcRef;
use deno_core::{
    error::{type_error, AnyError},
    op, AsyncRefCell, CancelFuture, CancelHandle, Canceled, OpState, Resource, ResourceId,
    ZeroCopyBuf,
};
use reqwest::{
    header::{HeaderName, HeaderValue, HOST},
    Method, Response, Url,
};
use serde::Serialize;
use tokio::io::AsyncReadExt;
use tokio_util::io::StreamReader;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchReture {
    request_rid: u32,
    cancel_handle_rid: u32,
}

type CancelableResponseResult = Result<Result<Response, AnyError>, Canceled>;

struct FetchRequestResource(Pin<Box<dyn Future<Output = CancelableResponseResult>>>);

impl Resource for FetchRequestResource {
    // fn close(self: Rc<Self>) {
    //     self.0.cancel()
    // }
}

struct FetchCancelHandle(Rc<CancelHandle>);

impl Resource for FetchCancelHandle {}

#[op]
pub fn op_fetch(
    state: &mut OpState,
    method: String,
    url: String,
    headers: Vec<(String, String)>,
) -> Result<FetchReture, AnyError> {
    let client = state.borrow::<reqwest::Client>();
    let method = Method::from_bytes(method.as_bytes()).unwrap();
    let url = Url::parse(&url).unwrap();
    let mut request = client.request(method, url);
    for (key, value) in headers {
        let name = HeaderName::from_bytes(&key.as_bytes())
            .map_err(|err| type_error(err.to_string()))
            .unwrap();
        let v = HeaderValue::from_bytes(&value.as_bytes())
            .map_err(|err| type_error(err.to_string()))
            .unwrap();
        if name != HOST {
            request = request.header(name, v);
        }
    }

    let cancel_handle = CancelHandle::new_rc();
    let cencel_handle_ = cancel_handle.clone();

    let fut = async move {
        request
            .send()
            .or_cancel(cencel_handle_)
            .await
            .map(|res| res.map_err(|err| type_error(err.to_string())))
    };
    let request_rid = state
        .resource_table
        .add(FetchRequestResource(Box::pin(fut)));
    let cancel_handle_rid = state.resource_table.add(FetchCancelHandle(cancel_handle));
    Ok(FetchReture {
        request_rid,
        cancel_handle_rid,
    })

    // todo(其他协议):
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchResponse {
    status: u16,
    status_text: String,
    headers: Vec<(String, String)>,
    url: String,
    request_rid: u32,
}

type BytesStream = Pin<Box<dyn Stream<Item = Result<bytes::Bytes, std::io::Error>> + Unpin>>;

struct FetchResponseBodyResource {
    reader: AsyncRefCell<StreamReader<BytesStream, bytes::Bytes>>,
    cancel: CancelHandle,
}

impl Resource for FetchResponseBodyResource {
    fn name(&self) -> Cow<str> {
        "fetchResponseBody".into()
    }

    fn read_return(self: Rc<Self>, mut buf: ZeroCopyBuf) -> AsyncResult<(usize, ZeroCopyBuf)> {
        Box::pin(async move {
            let mut reader = RcRef::map(&self, |r| &r.reader).borrow_mut().await;
            let cancel = RcRef::map(self, |r| &r.cancel);
            let read = reader.read(&mut buf).try_or_cancel(cancel).await?;
            Ok((read, buf))
        })
    }

    fn close(self: Rc<Self>) {
        self.cancel.cancel()
    }
}

#[op]
pub async fn op_fetch_send(
    state: &mut OpState,
    rid: ResourceId,
) -> Result<FetchResponse, AnyError> {
    let request = state.resource_table.take::<FetchRequestResource>(rid)?;

    let request = Rc::try_unwrap(request)
        .ok()
        .expect("multiple op_fetch_send ongoing");

    let result = match request.0.await {
        Ok(Ok(res)) => res,
        Ok(Err(err)) => return Err(type_error(err.to_string())),
        Err(_) => return Err(type_error("request was canceled")),
    };
    let status = result.status();
    let url = result.url().to_string();
    let mut res_headers = Vec::new();
    for (key, val) in result.headers().iter() {
        res_headers.push((key.as_str().into(), val.to_str().unwrap().into()));
    }
    let stream: BytesStream = Box::pin(
        result
            .bytes_stream()
            .map(|r| r.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))),
    );
    let stream_reader = StreamReader::new(stream);
    let data_rid = state.resource_table.add(FetchResponseBodyResource {
        reader: AsyncRefCell::new(stream_reader),
        cancel: CancelHandle::default(),
    });
    Ok(FetchResponse {
        status: status.as_u16(),
        url,
        headers: res_headers,
        status_text: status.canonical_reason().unwrap_or("").to_string(),
        request_rid: data_rid,
    })
}
