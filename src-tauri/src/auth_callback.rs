use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;
use std::time::Duration;
use tauri::Emitter;

const AUTH_CALLBACK_ADDRESS: &str = "127.0.0.1:45453";
const AUTH_CALLBACK_PATH: &str = "/auth/callback";
const AUTH_CALLBACK_EVENT: &str = "auth-callback";

pub struct AuthCallbackState {
    redirect_url: String,
    startup_error: Mutex<Option<String>>,
}

impl AuthCallbackState {
    fn available() -> Self {
        Self {
            redirect_url: format!("http://{AUTH_CALLBACK_ADDRESS}{AUTH_CALLBACK_PATH}"),
            startup_error: Mutex::new(None),
        }
    }

    fn unavailable(error: String) -> Self {
        Self {
            redirect_url: format!("http://{AUTH_CALLBACK_ADDRESS}{AUTH_CALLBACK_PATH}"),
            startup_error: Mutex::new(Some(error)),
        }
    }
}

#[tauri::command]
pub fn get_auth_redirect_url(state: tauri::State<'_, AuthCallbackState>) -> Result<String, String> {
    if let Some(error) = state
        .startup_error
        .lock()
        .map_err(|_| "The Google sign-in callback state is unavailable.".to_string())?
        .as_ref()
    {
        return Err(error.clone());
    }

    Ok(state.redirect_url.clone())
}

pub fn start(app: tauri::AppHandle) -> AuthCallbackState {
    let listener = match TcpListener::bind(AUTH_CALLBACK_ADDRESS) {
        Ok(listener) => listener,
        Err(error) => {
            let message = format!(
                "Smalltalk could not start the Google sign-in callback on {AUTH_CALLBACK_ADDRESS}: {error}"
            );
            eprintln!("[auth] {message}");
            return AuthCallbackState::unavailable(message);
        }
    };

    std::thread::spawn(move || {
        for connection in listener.incoming() {
            match connection {
                Ok(stream) => handle_connection(&app, stream),
                Err(error) => eprintln!("[auth] callback connection failed: {error}"),
            }
        }
    });

    AuthCallbackState::available()
}

fn handle_connection(app: &tauri::AppHandle, mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));

    let request_target = {
        let mut reader = BufReader::new(&stream);
        let mut request_line = String::new();
        if reader.read_line(&mut request_line).is_err() {
            return;
        }
        parse_request_target(&request_line).map(str::to_owned)
    };

    let Some(request_target) = request_target else {
        write_response(&mut stream, "400 Bad Request", ERROR_PAGE);
        return;
    };

    let path = request_target.split('?').next().unwrap_or_default();
    if path != AUTH_CALLBACK_PATH {
        write_response(&mut stream, "404 Not Found", NOT_FOUND_PAGE);
        return;
    }

    let query = request_target
        .split_once('?')
        .map(|(_, query)| query)
        .unwrap_or_default();
    let callback_url = if query.is_empty() {
        "smalltalk://auth/callback".to_string()
    } else {
        format!("smalltalk://auth/callback?{query}")
    };

    let page = if query_has_parameter(query, "code") {
        SUCCESS_PAGE
    } else {
        ERROR_PAGE
    };

    if let Err(error) = app.emit(AUTH_CALLBACK_EVENT, vec![callback_url]) {
        eprintln!("[auth] could not deliver callback to the app: {error}");
        write_response(
            &mut stream,
            "500 Internal Server Error",
            DELIVERY_ERROR_PAGE,
        );
        return;
    }

    write_response(&mut stream, "200 OK", page);
}

fn parse_request_target(request_line: &str) -> Option<&str> {
    let mut parts = request_line.split_whitespace();
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some("GET"), Some(target), Some(version), None)
            if version == "HTTP/1.1" || version == "HTTP/1.0" =>
        {
            Some(target)
        }
        _ => None,
    }
}

fn query_has_parameter(query: &str, expected_name: &str) -> bool {
    query.split('&').any(|part| {
        let (name, value) = part.split_once('=').unwrap_or((part, ""));
        name == expected_name && !value.trim().is_empty()
    })
}

fn write_response(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'; style-src 'unsafe-inline'\r\nReferrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

const SUCCESS_PAGE: &str = r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Google sign-in complete</title><style>
  body { margin: 0; min-height: 100vh; display: grid; place-items: center; background: #f5f4f1; color: #18251f; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
  main { width: min(440px, calc(100vw - 48px)); text-align: center; }
  .mark { width: 48px; height: 48px; margin: 0 auto 24px; display: grid; place-items: center; border-radius: 50%; background: #00693c; color: white; font-size: 26px; }
  h1 { margin: 0 0 12px; font-size: 28px; letter-spacing: -0.03em; }
  p { margin: 0; color: #5c6761; font-size: 16px; line-height: 1.55; }
</style></head><body><main><div class="mark">&#10003;</div><h1>Google sign-in complete</h1><p>Smalltalk is finishing your sign-in. You can close this tab and return to the app.</p></main></body></html>"#;

const ERROR_PAGE: &str = r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Smalltalk sign-in was not completed</title><style>
  body { margin: 0; min-height: 100vh; display: grid; place-items: center; background: #f5f4f1; color: #18251f; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
  main { width: min(440px, calc(100vw - 48px)); text-align: center; }
  .mark { width: 48px; height: 48px; margin: 0 auto 24px; display: grid; place-items: center; border-radius: 50%; background: #8b3a32; color: white; font-size: 24px; }
  h1 { margin: 0 0 12px; font-size: 28px; letter-spacing: -0.03em; }
  p { margin: 0; color: #5c6761; font-size: 16px; line-height: 1.55; }
</style></head><body><main><div class="mark">!</div><h1>Sign-in was not completed</h1><p>Return to Smalltalk to see what happened and try again.</p></main></body></html>"#;

const DELIVERY_ERROR_PAGE: &str = r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Could not return to Smalltalk</title></head><body><main><h1>Could not return to Smalltalk</h1><p>Keep Smalltalk open and try signing in again.</p></main></body></html>"#;
const NOT_FOUND_PAGE: &str = "<!doctype html><html><body><h1>Not found</h1></body></html>";

#[cfg(test)]
mod tests {
    use super::{parse_request_target, query_has_parameter};

    #[test]
    fn accepts_the_exact_callback_request() {
        assert_eq!(
            parse_request_target("GET /auth/callback?code=abc HTTP/1.1\r\n"),
            Some("/auth/callback?code=abc")
        );
    }

    #[test]
    fn rejects_non_get_and_malformed_requests() {
        assert_eq!(
            parse_request_target("POST /auth/callback HTTP/1.1\r\n"),
            None
        );
        assert_eq!(parse_request_target("GET /auth/callback\r\n"), None);
    }

    #[test]
    fn requires_a_non_empty_authorization_code() {
        assert!(query_has_parameter("code=abc&state=def", "code"));
        assert!(!query_has_parameter("code=&state=def", "code"));
        assert!(!query_has_parameter("error=access_denied", "code"));
    }
}
