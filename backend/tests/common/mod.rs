use core::fmt;
use std::{future::Future, net::TcpListener, time::Duration};

use actix_web::{App, HttpServer};
use awc::http::header::TryIntoHeaderPair;

pub use backend::constants::{
    access, http, json, output_channel, route, url, websocket, ws_action,
};
use backend::constants::{defaults, env};

pub const OWNER_NAME: &str = "owner";
pub const GUEST_NAME: &str = "guest";
pub const USER_NAME: &str = "user";
pub const TEST_FILE: &str = "test";
pub const TEST_PASSWORD: &str = "correct horse battery staple";
pub const HELLO_WORLD_OUTPUT: &str = "Hello World\n";
pub const TEST_INSERT_TEXT: &str = "hello world";
pub const INITIAL_REVISION: u64 = 0;
pub const AFTER_INSERT_REVISION: u64 = 1;
pub const DELETE_OPERATION: i32 = -5;
pub const RETAIN_OPERATION: i32 = 6;
pub const PASSWORD_BROADCAST_TIMEOUT_SECS: u64 = 3;
pub const WS_RECEIVE_TIMEOUT_MS: u64 = 500;
pub const TEST_SERVER_WORKERS: usize = 1;
pub const READY_ATTEMPTS: usize = 100;
pub const READY_RETRY_DELAY_MS: u64 = 10;

tokio::task_local! {
    static TEST_API_BASE: String;
}

struct TestServer {
    base_url: String,
    handle: actix_web::dev::ServerHandle,
}

impl TestServer {
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("Cannot bind test server");
        let address = listener
            .local_addr()
            .expect("Cannot read test server address");
        let app_data = backend::new_app_data();

        let server = HttpServer::new(move || {
            let app_data = app_data.clone();

            App::new()
                .wrap(backend::cors())
                .configure(move |config| app_data.configure(config))
        })
        .workers(TEST_SERVER_WORKERS)
        .listen(listener)
        .expect("Cannot start test server")
        .run();

        let handle = server.handle();
        actix_rt::spawn(server);

        let base_url = format!("http://{address}");
        wait_until_ready(&base_url).await;

        Self { base_url, handle }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let handle = self.handle.clone();
        actix_rt::spawn(async move {
            handle.stop(true).await;
        });
    }
}

async fn wait_until_ready(base_url: &str) {
    for _ in 0..READY_ATTEMPTS {
        if let Ok(response) = awc::Client::new()
            .get(format!("{base_url}{}", route::HEALTH))
            .send()
            .await
        {
            if response.status().is_success() {
                return;
            }
        }

        tokio::time::sleep(Duration::from_millis(READY_RETRY_DELAY_MS)).await;
    }

    panic!("Test server did not become ready at {base_url}");
}

pub async fn with_test_server<F, Fut, T>(test: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    if std::env::var_os(env::TEST_API_URL).is_some() {
        return test().await;
    }

    let server = TestServer::start().await;
    TEST_API_BASE.scope(server.base_url.clone(), test()).await
}

#[macro_export(local_inner_macros)]
macro_rules! _const {
    (API_URL) => {
        defaults::BIND_ADDRESS
    };
    (WS_URL) => {
        "ws://localhost:8080/ws"
    };
}

pub fn auth_header(token: impl fmt::Display) -> impl TryIntoHeaderPair {
    (
        http::AUTHORIZATION_HEADER,
        format!("{}{}", http::BEARER_PREFIX, token),
    )
}

pub fn api_url(path: &str) -> String {
    let base = api_base_url();
    format!("{base}{path}")
}

pub fn ws_url(path: &str) -> String {
    let base = api_base_url()
        .replacen(url::HTTP_SCHEME, url::WS_SCHEME, 1)
        .replacen(url::HTTPS_SCHEME, url::WSS_SCHEME, 1);
    format!("{base}{path}")
}

fn api_base_url() -> String {
    if let Ok(base) = std::env::var(env::TEST_API_URL) {
        return base;
    }

    TEST_API_BASE
        .try_with(Clone::clone)
        .unwrap_or_else(|_| format!("http://{}", defaults::BIND_ADDRESS))
}

#[macro_export]
macro_rules! request {
    ($path:expr, $($tt:tt)*) => { request!([get] $path, $($tt)*) };
    ([$method:ident] $path:expr, $($tt:tt)*) => {
        request!(@compose [::awc::Client::new().request(::awc::http::Method::$method, $crate::common::api_url($path))] $($tt)*)
    };
    (@compose [$base:expr]) => { $base };
    (@send [$base:expr] dbg $(, $($tt:tt)+)?) => {
        request!(@compose [{
            let response = $base;
            dbg!(&response);
            response
        }] $($($tt)+)?)
    };
    (@compose [$base:expr] as $auth:expr $(, $($tt:tt)+)?) => {
        request!(@compose [$base.insert_header($crate::common::auth_header($auth))] $($($tt)+)?)
    };

    (@compose [$base:expr] send $(, $($tt:tt)+)?) => {
        request!(@send [$base.send().await.unwrap()] $($($tt)+)?)
    };

    (@compose [$base:expr] send { $($body:tt)* } $(, $($tt:tt)+)?) => {
        request!(@send [$base.send_json(&::serde_json::json!({ $($body)* })).await.unwrap()] $($($tt)+)?)
    };

    (@send [$base:expr]) => { $base };
    (@send [$base:expr] dbg $(, $($tt:tt)+)?) => {
        request!(@send [{
            let response = $base;
            dbg!(&response);
            response
        }] $($($tt)+)?)
    };
    (@send [$base:expr] expect $code:literal $(, $($tt:tt)+)?) => {
        request!(@send [{
            let response = $base;
            assert_eq!(response.status(), $code as u16);
            response
        }] $($($tt)+)?)
    };

    (@send [$base:expr] expect $code:ident $(, $($tt:tt)+)?) => {
        request!(@send [{
            let response = $base;
            assert_eq!(response.status(), ::awc::http::StatusCode::$code);
            response
        }] $($($tt)+)?)
    };

    (@send [$base:expr] json $(, $($tt:tt)+)?) => {
        json_assert!($base.json::<::serde_json::Value>().await.expect("Should be a json"), $($($tt)+)?)
    };

}

#[macro_export]
macro_rules! ws {
    (connect $owner:expr, $project_id:expr $(, $password:expr)?) => {
        ::awc::Client::new()
            .ws($crate::common::ws_url(&format!("{}/{}", $crate::common::route::WS_BASE, $project_id)))
            .set_header($crate::common::http::SEC_WEBSOCKET_PROTOCOL_HEADER, ws!(@connect-protocol $owner $($password)?))
            .connect()
            .await
            .unwrap()
            .1
    };

    (@connect-protocol $owner:expr) => {format!("{}.{}", $crate::common::websocket::AUTH_PROTOCOL, $owner)};
    (@connect-protocol $owner:expr, $password:expr) => {format!("{}.{}, {}.{}", $crate::common::websocket::AUTH_PROTOCOL, $owner, $crate::common::websocket::PASSWORD_PROTOCOL, $password)};

    (recv $ws:ident, $name:expr $(, $($tt:tt)+)?) => {
        match ::tokio::time::timeout(::core::time::Duration::from_millis($crate::common::WS_RECEIVE_TIMEOUT_MS), $ws.next()).await {
            Ok(Some(Ok(awc::ws::Frame::Text(msg)))) =>  {
                json_assert!(
                    ::serde_json::from_slice::<::serde_json::Value>(&msg).unwrap(),
                    tee_ref
                    [ get $crate::common::json::ACTION, as string, eq $name ]
                    $($($tt)+)?
                )
            }
            Ok(Some(Ok(kind))) => panic!("Expecting Text. Should receive {}: {:?}", $name, kind),
            Ok(Some(Err(err))) => panic!("Error receiving message. Should receive {}: {}", $name, err),
            Ok(None) => panic!("No remaining messages. Should receive {}", $name),
            Err(_) => panic!("Timeout. Should receive {}", $name),
        }
    };

    (send $ws:ident, $action:expr, { $($tt:tt)* }) => {
        $ws
            .send(::awc::ws::Message::Text(::serde_json::json!({
                ($crate::common::json::ACTION): $action,
                $($tt)*
            }).to_string().into()))
            .await
            .unwrap()
    };
}

#[macro_export]
macro_rules! json_assert {
    ($base:expr $(, $($tt:tt)+)?) => { json_assert!(@json (Value) [$base] $($($tt)+)?) };

    (@json [$base:expr] $($tt:tt)*) => { json_assert!(@json (Value) [$base] $($tt)*) };
    (@json ($ty:ident) [$base:expr]) => { $base };
    (@json ($ty:ident) [$base:expr] tee_ref $([ $($tt:tt)+ ])+) => {{
        let response = $base;

        ($(
            json_assert!(@json ($ty) [&response] $($tt)+),
        )+)
    }};
    (@json (Value) [$base:expr] get $key:expr $(, $($tt:tt)+)?) => {
        json_assert!(@json (Value) [$key] [$base.get($key).expect(&format!("{} should exist in {}", $key, $base))] $($($tt)+)?)
    };
    (@json (Object) [$base:expr] get $key:expr $(, $($tt:tt)+)?) => {
        json_assert!(@json (Value) [$key] [$base.get($key).expect(&format!("{} should exist in {:?}", $key, $base))] $($($tt)+)?)
    };

    (@json ($ty:ident) [$base:expr] eq $eq:expr $(, $($tt:tt)+)?) => {
        json_assert!(@json ($ty) [{
            let base = $base;
            assert_eq!(base, $eq);
            base
        }] $($($tt)+)?)
    };
    (@json ($ty:ident) [$base:expr] dbg $(, $($tt:tt)+)?) => {
        json_assert!(@json ($ty) [{
            let base = $base;
            println!(concat!("[", file!(), ":", line!(), ":", column!(), "] {:?}"), base);
            base
        }] $($($tt)+)?)
    };

    (@json ($ty:ident) [$key:expr] [$base:expr]) => { $base };

    (@json ($ty:ident) [$key:expr] [$base:expr] eq $eq:expr $(, $($tt:tt)+)?) => {
        json_assert!(@json ($ty) [$key] [{
            let base = $base;
            assert_eq!(base, $eq, "{}", $key);
            base
        }] $($($tt)+)?)
    };

    (@json ($ty:ident) [$key:expr] [$base:expr] dbg $(, $($tt:tt)+)?) => {
        json_assert!(@json ($ty) [$key] [{
            let base = $base;
            println!(
                "[{}:{}:{}] {} = {:?}",
                file!(),
                line!(),
                column!(),
                $key,
                base
            );
            base
        }] $($($tt)+)?)
    };

    (@json (Array) [$key:expr] [$base:expr] expect empty $(, $($tt:tt)+)?) => {
        json_assert!(@json (Array) [$key] [{
            let base = $base;
            if !base.is_empty() {
                panic!("{} should be empty", $key)
            }
            base
        }] $($($tt)+)?)
    };

    (@json (Value) [$key:expr] [$base:expr] as array $(, $($tt:tt)+)?) => {
        json_assert!(
            @json
            (Array)
            [$key]
            [$base
                .as_array()
                .expect(&format!("{} should be an array", $key))
                .clone()]
            $($($tt)+)?
        )
    };

    (@json (Value) [$key:expr] [$base:expr] as unsigned $(, $($tt:tt)+)?) => {
        json_assert!(
            @json
            (Unsigned)
            [$key]
            [$base
                .as_u64()
                .expect(&format!("{} should be an unsigned number", $key))]
            $($($tt)+)?
        )
    };

    (@json (Value) [$key:expr] [$base:expr] as string $(, $($tt:tt)+)?) => {
        json_assert!(
            @json
            (String)
            [$key]
            [$base
                .as_str()
                .expect(&format!("{} should be a string", $key))
                .to_string()]
            $($($tt)+)?
        )
    };

    (@json (Value) [$key:expr] [$base:expr] as object $(, $($tt:tt)+)?) => {
        json_assert!(
            @json
            (Object)
            [$base
                .as_object()
                .expect(&format!("{} should be an object", $key))
                .clone()]
            $($($tt)+)?
        )
    };
}
