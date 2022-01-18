use deno_core::error::AnyError;
use hyper::http::response::Parts;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use routerify::prelude::RequestExt;
use routerify::{RequestInfo, Router, RouterService};
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};
use std::num::ParseIntError;
use std::str::{FromStr, ParseBoolError};
use three_em_arweave::arweave::Arweave;
use three_em_arweave::cache::{ArweaveCache, CacheExt};
use three_em_executor::execute_contract;
use three_em_executor::executor::ExecuteResult;
use url::Url;

pub struct ServerConfiguration {
  pub port: u16,
  pub host: IpAddr,
}

pub fn build_error(message: &str) -> Response<Body> {
  Response::builder()
    .status(400)
    .body(Body::from(
      serde_json::json!({
        "status": 400,
        "message": message})
      .to_string(),
    ))
    .unwrap()
}

async fn echo(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
  match (req.method(), req.uri().path()) {
        (&Method::GET, "/evaluate") => {
            let params: HashMap<String, String> = req
                .uri()
                .query()
                .map(|v| {
                    url::form_urlencoded::parse(v.as_bytes())
                        .into_owned()
                        .collect()
                })
                .unwrap_or_else(HashMap::new);

            let contract_id = params.get("contractId").map(|i| i.to_owned());
            let height = params.get("height").map(|i| i.to_owned());
            let gateway_host = params.get("gatewayHost").map(|i| i.to_owned()).unwrap_or(String::from("arweave.net"));
            let gateway_port = params.get("gatewayPort").map(|i| i.to_owned()).unwrap_or(String::from("443"));
            let gateway_protocol = params.get("gatewayProtocol").map(|i| i.to_owned()).unwrap_or(String::from("https"));
            let show_validity = params.get("showValidity").map(|i| i.to_owned()).unwrap_or(String::from("false"));
            let cache = params.get("cache").map(|i| i.to_owned()).unwrap_or(String::from("false"));
            let show_errors = params.get("showErrors").map(|i| i.to_owned()).unwrap_or(String::from("false"));

            let (height_invalid, height) = {
                if let Some(h) = height {
                    let height_maybe: Result<usize, ParseIntError> = usize::from_str(h.as_str());
                    if let Ok(provided_height) = height_maybe {
                        (false, Some(provided_height))
                    } else {
                        (true, None)
                    }
                } else {
                    (false, None)
                }
            };

            let show_validity = {
                let show_validity_maybe: Result<bool, ParseBoolError> = bool::from_str(show_validity.as_str());
                if let Ok(provided_validity) = show_validity_maybe {
                    Some(provided_validity)
                } else {
                    None
                }
            };

            let port = {
                let port_maybe: Result<i32, ParseIntError> = i32::from_str(gateway_port.as_str());
                if let Ok(provided_port) = port_maybe {
                    Some(provided_port)
                } else {
                    None
                }
            };

            let cache = {
                let maybe_cache: Result<bool, ParseBoolError> = bool::from_str(cache.as_str());
                if let Ok(provided_cache) = maybe_cache {
                    Some(provided_cache)
                } else {
                    None
                }
            };

            let show_errors = {
                let maybe_show_errors: Result<bool, ParseBoolError> = bool::from_str(show_errors.as_str());
                if let Ok(provided_show_errors) = maybe_show_errors {
                    Some(provided_show_errors)
                } else {
                    None
                }
            };

            let mut response_result: Option<Response<Body>> = None;

            if contract_id.is_none() {
                response_result = Some(build_error("contractId was not provided in query parameters. A contract id must be provided."));
            } else if height_invalid {
                response_result = Some(build_error("The height provided is incorrect. Please provide a valid numeric value."));
            } else if port.is_none() {
                response_result = Some(build_error("The gateway port provided is invalid. Please provided a valid numeric value."));
            } else if show_validity.is_none() {
                response_result = Some(build_error("The 'showValidity' parameter contains wrong data. Please provided a valid boolean value."));
            } else if cache.is_none() {
                response_result = Some(build_error("The 'cache' parameter contains wrong data. Please provided a valid boolean value."));
            } else if show_errors.is_none() {
                response_result = Some(build_error("The 'showErrors' parameter contains wrong data. Please provided a valid boolean value."));
            } else {
                 let arweave = Arweave::new(port.unwrap(), gateway_host.to_owned(), gateway_protocol.to_owned(), ArweaveCache::new());
                 let execute_result = execute_contract(&arweave, contract_id.unwrap().to_owned(), None, None, height, cache.unwrap(), show_errors.unwrap()).await;
                // match execute_result {
                //     Ok(result) => {
                //         match result {
                //             ExecuteResult::V8(val, validity) => {
                //                 if show_validity.unwrap() {
                //                     response_result = Some(Response::new(Body::from(
                //                         serde_json::json!({
                //                             "state": val,
                //                             "validity": validity
                //                         }).to_string()
                //                     )));
                //                 } else {
                //                     response_result = Some(Response::new(Body::from(
                //                         serde_json::json!({
                //                             "state": val
                //                         }).to_string()
                //                     )));
                //                 }
                //             },
                //             ExecuteResult::Evm(_, _, _) => {
                //                 response_result = Some(build_error("EVM evaluation is disabled"));
                //             }
                //         }
                //     },
                //     Err(e) => {
                //         response_result = Some(build_error(e.to_string().as_str()));
                //     }
                // }
            }

            Ok(response_result.unwrap())
        }
        _ => {
            Ok(Response::new(Body::from(
                "Try POSTing data to /echo such as: `curl localhost:3000/echo -XPOST -d 'hello world'`",
            )))
        }
    }
}

pub async fn start_local_server(config: ServerConfiguration) {
  let addr = SocketAddr::from((config.host, config.port));
  let service =
    make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(echo)) });

  let server = Server::bind(&addr).executor(LocalExec).serve(service);
  server.await.unwrap();
}

#[derive(Clone, Copy, Debug)]
struct LocalExec;

impl<F> hyper::rt::Executor<F> for LocalExec
where
  F: std::future::Future + 'static, // not requiring `Send`
{
  fn execute(&self, fut: F) {
    // This will spawn into the currently running `LocalSet`.
    tokio::task::spawn_local(fut);
  }
}
