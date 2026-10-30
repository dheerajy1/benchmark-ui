# Backend CORS requirement

Status: **not applied**. `benchmark-api` was inspected only and left unchanged.

## What was verified in benchmark-api

Found by reading the source, the backend was not started.

| Check | Result |
| --- | --- |
| `src/app.rs` router layers | None. The router is `v1::router()` plus a 404 fallback, no CORS or other middleware. |
| `Cargo.toml` | No `tower-http` or any other CORS crate. |
| Source search for `access-control`, `origin`, `OPTIONS` | No matches. |
| Route fallbacks | Every route ends in `.fallback(method_not_allowed)`, so an `OPTIONS` request gets `405 METHOD_NOT_ALLOWED`. |
| Listener | `main.rs` binds `127.0.0.1:<APP_PORT>` only (default `5001`). |

## Resulting browser behaviour

The dashboard is served from `http://localhost:5002` in development and port `8092` in production. The API is on port `5001`, so every request is cross-origin.

* **GET requests** (overview, history, detail, comparison) are sent, but the response has no `Access-Control-Allow-Origin` header. The browser hides the response from the page and `fetch` fails. The dashboard reports this as "API unreachable" and mentions CORS.
* **POST /benchmarks** sends `Content-Type: application/json`, which triggers a preflight `OPTIONS`. That gets a 405 without CORS headers, so the browser never sends the POST.

`localhost` and `127.0.0.1` are different origins. Open the dashboard on the origin you allow.

## Required change in benchmark-api

Add the dependency:

```toml
tower-http = { version = "0.6", features = ["cors"] }
```

Wrap the router in `src/app.rs`. The layer goes last so the fallback and 405 responses also carry the headers:

```rust
use axum::http::{header::CONTENT_TYPE, HeaderValue, Method};
use tower_http::cors::CorsLayer;

let cors = CorsLayer::new()
    .allow_origin([
        HeaderValue::from_static("http://localhost:5002"),
        HeaderValue::from_static("http://127.0.0.1:5002"),
        HeaderValue::from_static("http://localhost:8092"),
        HeaderValue::from_static("http://127.0.0.1:8092"),
    ])
    .allow_methods([Method::GET, Method::POST])
    .allow_headers([CONTENT_TYPE]);

Router::new()
    .merge(v1::router())
    .fallback(fallback::not_found)
    .with_state(AppState { env, repository })
    .layer(cors)
```

The API uses no cookies or credentials, so `allow_credentials` is not needed. Prefer an explicit origin list over `Any`.

## Frontend configuration

`API_BASE_URL` in `.env.development`, `.env.production` and `.env.example` sets the API address. It is read at build time and is public. The backend listens on loopback only, so use `http://127.0.0.1:5001` or `http://localhost:5001`.
