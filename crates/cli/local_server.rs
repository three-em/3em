use deno_core::error::AnyError;
use hyper::http::response::Parts;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use indoc::indoc;
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

           let height = height.map(|h| h.parse::<usize>().unwrap_or(usize::MAX));
           let show_validity = show_validity.parse::<bool>().unwrap_or(false);
           let cache = cache.parse::<bool>().unwrap_or(false);
           let show_errors = show_errors.parse::<bool>().unwrap_or(false);
           let port = gateway_port.parse::<i32>().unwrap_or(443);
           let mut response_result: Option<Response<Body>> = None;

           if contract_id.is_none() {
              response_result = Some(build_error("contractId was not provided in query parameters. A contract id must be provided."));
            } else {
                 let arweave = Arweave::new(port, gateway_host.to_owned(), gateway_protocol.to_owned(), ArweaveCache::new());
                 let execute_result = execute_contract( contract_id.unwrap().to_owned(), height, cache, show_errors, None, None, &arweave).await;
                match execute_result {
                     Ok(result) => {
                         match result {
                             ExecuteResult::V8(data) => {
                                 let val = data.state;
                                 let validity = data.validity;
                                 let result = data.result.unwrap_or(serde_json::Value::Null);
                                 if show_validity {
                                     response_result = Some(Response::new(Body::from(
                                         serde_json::json!({
                                             "state": val,
                                             "validity": validity,
                                             "result": result
                                         }).to_string()
                                     )));
                                 } else {
                                     response_result = Some(Response::new(Body::from(
                                         serde_json::json!({
                                             "state": val,
                                             "result": result
                                         }).to_string()
                                     )));
                                 }
                             },
                             ExecuteResult::Evm(_, _, _) => {
                                 response_result = Some(build_error("EVM evaluation is disabled"));
                             }
                         }
                     },
                     Err(e) => {
                         response_result = Some(build_error(e.to_string().as_str()));
                     }
                 }
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

  println!("Serving {}", addr.to_string());
  println!(
    "{}",
    indoc! {
    "
       Endpoints:
         GET   /evaluate   Evaluates a contract state given a contract id
               ?contractId   Id of contract to be evaluated   (Required)   [string]
               ?height   Height to be used during evaluation   [number]
               ?gatewayHost   Gateway to be used for and during evaluation   (Default: arweave.net)   [string]
               ?gatewayPort   Port to be used for gateway communication   (Default: 443)   [number]
               ?gatewayProtocol   Protocol to be used for gateway communication   (Default: https)   [string]
               ?showValidity   Whether validity table should be included in the JSON response   (Default: false)   [boolean]
               ?cache   Whether built-in cache system should be used during execution   (Default: true)   [boolean]
               ?showErrors   Whether server console should print out execution exceptions   (Default: false)   [boolean]
      "}
  );

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
