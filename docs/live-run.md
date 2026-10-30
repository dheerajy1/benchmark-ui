# Live run page

Starting a benchmark now opens `/runs/live/:run_id`, a live view of the run. It follows the
API described in `docs/live-runs-api.md` of benchmark-api.

## Flow

1. The form posts to `POST /api/v1/live-runs`. The API answers `202` with the run id at once.
2. The app navigates to `/runs/live/:run_id`.
3. The page opens a WebSocket to `/api/v1/live-runs/{run_id}/ws?after=<last seq>` and applies
   every event to one state (`features/live/state.rs`).
4. When the run ends the page shows the outcome and a **View run** button to the stored run
   (`/runs/:run_id`). It does not redirect on its own, so the final numbers stay readable.

If another benchmark is already running, the form shows the API's `409` and, when the running
benchmark is a live run, a **Watch the active run** button.

## What the page shows

- A run bar: status, short id, target directory, elapsed time.
- A pipeline of the seven steps: detect runtime, measure artifact size, build, start and wait
  until ready, baseline, workload, save result. A step is waiting, running (with its elapsed
  time), done (with its duration), failed, skipped (with the reason) or not requested.
- During the workload: a progress bar with the numbers the API can really know (requests done
  of the requested total, or time elapsed of the planned duration), live tiles (memory, CPU,
  throughput, requests done and failed, workload time) and three charts of the most recent
  minute of samples (memory, CPU, throughput).
- An event list, newest first.
- At the end: the stored headline numbers, or the reason the run failed.

Nothing is invented. A value the API has not measured yet is shown as missing, there is no overall
percentage because build and startup have unknown durations, and latency percentiles only appear
in the final result because the API only knows them when the workload has finished.

## Connection handling

| Situation | What the page does |
|---|---|
| Connection lost | Reconnects with `after=<last seq>` (0.5 s, 1 s, 2 s, 4 s, 8 s), nothing is missed |
| Three connections in a row never open | Polls `GET /api/v1/live-runs/{run_id}?after=` every 2 s and says so |
| Events missing (a gap in the sequence) | Reconnects with `after=<last seq>` |
| Duplicate events | Ignored |
| Events of another run | Never applied |
| Close `4404` (the live record is gone) | Reads the stored run: shown as completed if stored, otherwise "no longer available" |
| Close `4429` (too many viewers) | Says so and tries again after 5 s |
| API unreachable | Says so with a **Try again** button |

Leaving the page closes the socket. The run is not affected by any of this: it keeps running and
is stored whether or not anybody watches.

## Request headers

The form has two optional boxes, one header per line as `Name: value`:

- Readiness headers: sent with the readiness probe. Empty means the API uses its own default health
  credentials.
- Request headers: sent with every request of the workload. A target that needs authentication needs them,
  otherwise every request fails and the throughput measures how fast the target says no.

Values are never stored by the API or by the page, and a validation message never repeats one.

## Configuration

No new setting. The WebSocket address is derived from `API_BASE_URL`: `https://` becomes `wss://`
and `http://` becomes `ws://`. A page served over HTTPS needs an `https://` API URL, because a
browser blocks an insecure WebSocket from a secure page.

The API checks the `Origin` of the handshake against its allow-list. `*.usw3.devtunnels.ms` and
`*.trycloudflare.com` origins pass. A dashboard on another origin gets a `403`, the page then falls
back to polling.

## Not verified

Whether a WebSocket works through VS Code dev tunnels and Cloudflare Tunnel has to be tested with a
real browser and a real run through the actual tunnel. Until then the polling fallback is the safety
net, not a promise.
