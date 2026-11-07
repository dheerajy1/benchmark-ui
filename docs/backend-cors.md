# Backend CORS requirement

Status: **applied in benchmark-api**. This page records what the dashboard needs from it. The contract is the CORS
section of `docs/api.md` in benchmark-api.

## What the API allows

| Setting | Value |
| --- | --- |
| Origins | `http://localhost:5002`, `http://127.0.0.1:5002` (development), `http://localhost:8092`, `http://127.0.0.1:8092` (production), `http://localhost:5003`, `http://127.0.0.1:5003` (development of this dashboard), `http://localhost:8093`, `http://127.0.0.1:8093` (production of this dashboard), `*.usw3.devtunnels.ms` and `*.trycloudflare.com` |
| Methods | `GET`, `POST`, `DELETE` |
| Request header | `Content-Type` |
| Exposed response header | `x-request-id` |

The same origin list protects the WebSocket handshake of the live page.

## Why the exposed header matters

A browser hides every response header that is not exposed. The dashboard reads `x-request-id` to show the request id in
the error panel and in the copied technical details. Without `Access-Control-Expose-Headers: x-request-id` the id is
missing there and the two sides cannot be matched in the API log.

## Resulting browser behaviour

The dashboard runs on port `5003` in development and `8093` in production, the API on port `5001`, so every request is
cross-origin.

* A call from an origin outside the list fails in the browser. The dashboard reports it as "API unreachable" and mentions CORS.
* `localhost` and `127.0.0.1` are different origins. Open the dashboard on the origin you want, both are allowed.
* A POST with `Content-Type: application/json` triggers a preflight `OPTIONS`, which the allow-list answers.
* The API uses no cookies or credentials.

## Frontend configuration

`API_BASE_URL` in `.env.development`, `.env.production` and `.env.example` sets the API address. It is read at build time
and is public. The backend listens on loopback only, so use `http://127.0.0.1:5001` or `http://localhost:5001`.
