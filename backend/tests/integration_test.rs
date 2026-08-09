mod common;

use futures_util::{sink::SinkExt, stream::StreamExt};
use std::time::Duration;

async fn login_as(guest_name: &str) -> (String, String) {
    request!([POST] common::route::AUTH_GUEST,
        send { (common::json::GUEST_NAME): guest_name },
        expect OK,
        json,
        tee_ref
        [ get common::json::JWT, as string ]
        [ get common::json::ID, as string ]
    )
}

async fn create_project(token: &str) -> String {
    request!([POST] common::route::PROJECT_CREATE,
        as token,
        send,
        expect CREATED,
        json,
        get common::json::ID,
        as string
    )
}

#[actix_rt::test]
async fn test_flow_two_users() {
    common::with_test_server(test_flow_two_users_body).await;
}

async fn test_flow_two_users_body() {
    // --- 1. Log in for both users ---
    let (owner, owner_id) = login_as(common::OWNER_NAME).await;
    let (guest, guest_id) = login_as(common::GUEST_NAME).await;

    // --- 2. Create project as "owner" ---
    let project_id = create_project(&owner).await;

    // --- 3. Connect both websockets ---
    let mut owner_ws = ws!(connect owner, &project_id);

    // Owner handshake
    {
        let (_, users) =
            ws!(recv owner_ws, common::ws_action::WELCOME, [ get common::json::USERS, as object ]);
        assert!(users.contains_key(&owner_id), "Self should be included")
    }

    // Owner get notified about its own connection
    _ = ws!(recv owner_ws, common::ws_action::USER_CONNECTED, [ get common::json::USER_ID, as string, eq owner_id ]);

    let mut guest_ws = ws!(connect guest, &project_id);

    // Guest handshake
    {
        let (_, users, session_id) = ws!(recv guest_ws, common::ws_action::WELCOME, [ get common::json::USERS, as object ] [ get common::json::SESSION_ID, as string ]);
        assert!(users.contains_key(&guest_id), "Self should be included");
        let _ = session_id;
    };

    // Get notified about guest connection
    _ = ws!(recv guest_ws, common::ws_action::USER_CONNECTED, [ get common::json::USER_ID, as string, eq guest_id ]);
    _ = ws!(recv owner_ws, common::ws_action::USER_CONNECTED, [ get common::json::USER_ID, as string, eq guest_id ]);

    // --- 4. Gives editor to guest ---
    ws!(send owner_ws, common::ws_action::PERMIT_ACCESS, {
        (common::json::USER_ID): guest_id,
        (common::json::ACCESS): common::access::EDITOR
    });

    // Access update
    _ = ws!(recv owner_ws, common::ws_action::UPDATE_ACCESS, [ get common::json::USER_ID, as string, eq guest_id ] [ get common::json::ACCESS, as string, eq common::access::EDITOR ]);
    _ = ws!(recv guest_ws, common::ws_action::UPDATE_ACCESS, [ get common::json::USER_ID, as string, eq guest_id ] [ get common::json::ACCESS, as string, eq common::access::EDITOR ]);

    // --- 5. Inserts text in "test" file ---
    ws!(send guest_ws, common::ws_action::FILE_CREATE, {
        (common::json::FILE): common::TEST_FILE
    });

    _ = ws!(recv owner_ws, common::ws_action::PROJECT_FILES, [ get common::json::FILES, as object, tee_ref [ get backend::constants::project::MAIN_FILE, as object ] [ get common::TEST_FILE, as object ] ] );
    _ = ws!(recv guest_ws, common::ws_action::PROJECT_FILES, [ get common::json::FILES, as object, tee_ref [ get backend::constants::project::MAIN_FILE, as object ] [ get common::TEST_FILE, as object ] ] );

    ws!(send guest_ws, common::ws_action::SYNC, {
        (common::json::REVISION): common::INITIAL_REVISION,
        (common::json::FILE): common::TEST_FILE,
        (common::json::ACTIONS): [common::TEST_INSERT_TEXT],
    });

    _ = ws!(recv owner_ws, common::ws_action::SYNC, [ get common::json::ACTIONS, as array ] [ get common::json::FILE, as string, eq common::TEST_FILE ] [ get common::json::REVISION, as unsigned, eq common::INITIAL_REVISION ]);
    _ = ws!(recv guest_ws, common::ws_action::SYNC, [ get common::json::ACTIONS, as array ] [ get common::json::FILE, as string, eq common::TEST_FILE ] [ get common::json::REVISION, as unsigned, eq common::INITIAL_REVISION ]);

    // --- 6. Guest deletes text in "test" file ---
    ws!(send guest_ws, common::ws_action::SYNC, {
        (common::json::REVISION): common::AFTER_INSERT_REVISION,
        (common::json::FILE): common::TEST_FILE,
        (common::json::ACTIONS): [common::DELETE_OPERATION, common::RETAIN_OPERATION],
    });

    _ = ws!(recv owner_ws, common::ws_action::SYNC, [ get common::json::ACTIONS, as array ] [ get common::json::FILE, as string, eq common::TEST_FILE ] [ get common::json::REVISION, as unsigned, eq common::AFTER_INSERT_REVISION ]);
    _ = ws!(recv guest_ws, common::ws_action::SYNC, [ get common::json::ACTIONS, as array ] [ get common::json::FILE, as string, eq common::TEST_FILE ] [ get common::json::REVISION, as unsigned, eq common::AFTER_INSERT_REVISION ]);

    // --- 7. Guest deletes "test" file ---
    ws!(send guest_ws, common::ws_action::FILE_DELETE, {
        (common::json::FILE): common::TEST_FILE
    });

    _ = ws!(recv owner_ws, common::ws_action::PROJECT_FILES, [ get common::json::FILES, as object, tee_ref [ get backend::constants::project::MAIN_FILE, as object ] ] );
    _ = ws!(recv guest_ws, common::ws_action::PROJECT_FILES, [ get common::json::FILES, as object, tee_ref [ get backend::constants::project::MAIN_FILE, as object ] ] );
}

#[actix::test]
async fn test_run_code() {
    common::with_test_server(test_run_code_body).await;
}

async fn test_run_code_body() {
    let (user, user_id) = login_as(common::USER_NAME).await;

    let project_id = create_project(&user).await;

    let mut user_ws = ws!(connect user, &project_id);

    // Handshake
    {
        let (_, users) =
            ws!(recv user_ws, common::ws_action::WELCOME, [ get common::json::USERS, as object ]);
        assert!(users.contains_key(&user_id), "Self should be included")
    }

    _ = ws!(recv user_ws, common::ws_action::USER_CONNECTED, [ get common::json::USER_ID, as string, eq user_id ]);

    ws!(send user_ws, common::ws_action::EXECUTE, { });

    _ = ws!(recv user_ws, common::ws_action::SYNC_OUTPUT_START);

    // Expect just a "Hello World" in stdout
    let (_, _, buf) = ws!(recv user_ws, common::ws_action::SYNC_OUTPUT, [get common::json::CHANNEL, as string, eq common::output_channel::STDOUT] [get common::json::BUF, as array]);

    let buf = buf
        .into_iter()
        .filter_map(|n| n.as_u64())
        .map(|n| n as u8)
        .collect::<Vec<u8>>();

    let buf = String::from_utf8_lossy(&buf);

    assert_eq!(buf, common::HELLO_WORLD_OUTPUT);

    _ = ws!(recv user_ws, common::ws_action::SYNC_OUTPUT_END);
}

#[actix_rt::test]
async fn test_lsp_initialize() {
    common::with_test_server(test_lsp_initialize_body).await;
}

async fn test_lsp_initialize_body() {
    let (user, user_id) = login_as(common::USER_NAME).await;
    let project_id = create_project(&user).await;
    let mut user_ws = ws!(connect user, &project_id);

    _ = ws!(recv user_ws, common::ws_action::WELCOME);
    _ = ws!(recv user_ws, common::ws_action::USER_CONNECTED, [
        get common::json::USER_ID, as string, eq user_id
    ]);

    ws!(send user_ws, common::ws_action::LSP, {
        (common::json::MESSAGE): ::serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": ::serde_json::Value::Null,
                "clientInfo": { "name": "rsground-test" },
                "rootUri": "file:///home",
                "capabilities": {}
            }
        })
    });

    let response = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let Some(Ok(awc::ws::Frame::Text(message))) = user_ws.next().await else {
                continue;
            };
            let value = ::serde_json::from_slice::<::serde_json::Value>(&message).unwrap();
            if value
                .get(common::json::ACTION)
                .and_then(|action| action.as_str())
                == Some(common::ws_action::LSP)
                && value
                    .get(common::json::MESSAGE)
                    .and_then(|message| message.get("id"))
                    .and_then(|id| id.as_u64())
                    == Some(1)
            {
                break value;
            }
        }
    })
    .await
    .expect("Rust Analyzer should initialize through the WebSocket bridge");

    assert!(response
        .get(common::json::MESSAGE)
        .and_then(|message| message.get("result"))
        .and_then(|result| result.get("capabilities"))
        .is_some_and(|capabilities| capabilities.is_object()));
}

#[actix_rt::test]
async fn test_project_password_is_not_disclosed() {
    common::with_test_server(test_project_password_is_not_disclosed_body).await;
}

async fn test_project_password_is_not_disclosed_body() {
    let (owner, _) = login_as(common::OWNER_NAME).await;
    let project_id = create_project(&owner).await;

    let mut owner_ws = ws!(connect owner, &project_id);
    _ = ws!(recv owner_ws, common::ws_action::WELCOME);
    _ = ws!(recv owner_ws, common::ws_action::USER_CONNECTED);

    ws!(send owner_ws, common::ws_action::CONFIG, {
        (common::json::PASSWORD): common::TEST_PASSWORD
    });
    tokio::time::timeout(
        Duration::from_secs(common::PASSWORD_BROADCAST_TIMEOUT_SECS),
        async {
            loop {
                let Some(Ok(awc::ws::Frame::Text(msg))) = owner_ws.next().await else {
                    continue;
                };
                let value = ::serde_json::from_slice::<::serde_json::Value>(&msg).unwrap();
                if value
                    .get(common::json::ACTION)
                    .and_then(|value| value.as_str())
                    == Some(common::ws_action::PROJECT_CONFIG)
                {
                    break;
                }
            }
        },
    )
    .await
    .expect("project password update should be broadcast");

    let (guest, _) = login_as(common::GUEST_NAME).await;
    let project_url = common::api_url(&format!("{}/{}", common::route::PROJECT, project_id));

    let response = ::awc::Client::new()
        .get(&project_url)
        .insert_header(common::auth_header(&guest))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), awc::http::StatusCode::UNAUTHORIZED);

    let mut response = ::awc::Client::new()
        .get(&project_url)
        .insert_header(common::auth_header(&guest))
        .insert_header((common::http::PROJECT_PASSWORD_HEADER, common::TEST_PASSWORD))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.body().await.unwrap();
    assert_eq!(
        status,
        awc::http::StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&body)
    );

    let body = ::serde_json::from_slice::<::serde_json::Value>(&body).unwrap();
    assert!(body.get(common::json::PASSWORD).is_none());
    assert_eq!(
        body.get(common::json::HAS_PASSWORD)
            .and_then(|value| value.as_bool()),
        Some(true)
    );
}

#[actix_rt::test]
async fn test_auth_rejects_invalid_guest_names_and_exposes_me() {
    common::with_test_server(test_auth_rejects_invalid_guest_names_and_exposes_me_body).await;
}

async fn test_auth_rejects_invalid_guest_names_and_exposes_me_body() {
    request!([POST] common::route::AUTH_GUEST,
        send { (common::json::GUEST_NAME): "   " },
        expect BAD_REQUEST
    );

    let too_long_name = "a".repeat(backend::constants::limits::MAX_GUEST_NAME_CHARS + 1);
    request!([POST] common::route::AUTH_GUEST,
        send { (common::json::GUEST_NAME): too_long_name },
        expect BAD_REQUEST
    );

    let (token, user_id) = login_as(common::USER_NAME).await;
    let mut response = request!([GET] common::route::AUTH_ME,
        as token,
        send,
        expect OK
    );
    let me = ::serde_json::from_slice::<::serde_json::Value>(&response.body().await.unwrap())
        .expect("auth/me should return JSON");
    assert_eq!(
        me.get(common::json::ID).and_then(|value| value.as_str()),
        Some(user_id.as_str())
    );
    assert_eq!(
        me.get(common::json::IS_GUEST)
            .and_then(|value| value.as_bool()),
        Some(true)
    );

    let invalid = ::awc::Client::new()
        .get(common::api_url(common::route::AUTH_ME))
        .insert_header(common::auth_header("invalid-token"))
        .send()
        .await
        .unwrap();
    assert_eq!(invalid.status(), awc::http::StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn test_project_name_validation() {
    common::with_test_server(test_project_name_validation_body).await;
}

async fn test_project_name_validation_body() {
    let (token, _) = login_as(common::OWNER_NAME).await;
    let too_long_name = "a".repeat(backend::constants::limits::MAX_PROJECT_NAME_CHARS + 1);

    let response = ::awc::Client::new()
        .post(common::api_url(&format!("/create/{too_long_name}")))
        .insert_header(common::auth_header(token))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), awc::http::StatusCode::BAD_REQUEST);
}
