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

## Cancelling a run

A running run shows a **Cancel run** button in the run bar. One click asks "Cancel this run?" with **Yes, cancel** and
**Keep running**, in place, with no dialog.

- It is only there while the run is running and this tab holds the right to cancel it. That right is a token the API
  gives once when the run starts. The form keeps it in this tab's `sessionStorage` under the run id, so a refresh keeps
  the button, and closing the tab discards it. If the storage is blocked the token is kept in memory for this page load only.
  The token is never put in a URL, shown, logged, or sent anywhere but the cancel request.
- A run started from another tab or browser shows "Only the tab that started this run can cancel it." instead of a button.
- After the request is taken the page shows "Cancelling, stopping the target…" and the running step says "Stopping". The run
  is **not** reported as cancelled until the API says so: that event comes once the build or the target and all its
  processes are really gone. Other viewers of the same run see the same.
- A cancelled run shows a "Benchmark cancelled" card with the step it stopped in, and "nothing was stored". The step is marked
  cancelled in the pipeline.
- If the API refuses, the page says why: the run already finished or is already saving its result (cancelling is no longer
  possible), the run is gone, or this browser has no right to cancel it (the token is dropped). A request that did not get
  through can be tried again.
- Closing the page never cancels a run. Only the button does.
- The token is removed from the tab as soon as the run ends.

### What was verified in a real browser

Observed on 2026-10-17 by hand, in one browser, dashboard through a VS Code dev tunnel, API behind a Cloudflare quick tunnel,
target kafka-api-gateway:

- Cancel during the build: the page showed Cancelled, "Stopped during the build step", nothing stored. No process of the project
  and no listener on the target's port were left.
- Cancel during the workload: events "Cancellation requested, stopping Workload" then "Run cancelled, nothing was stored". No
  process and no listener were left.
- A second tab on the same run showed "Only the tab that started this run can cancel it." and followed the run to Cancelled when
  the first tab cancelled it.

Not verified in a browser: cancel during startup (the startup attempts ended on their own first), refresh then cancel (the run
failed at startup before it could be cancelled), the double click on "Yes, cancel", other browsers and phones.

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

## What was verified in a real browser, and what was not

Verified on 2026-10-13, in one browser, with the dashboard served through a VS Code dev tunnel and
the API behind a Cloudflare quick tunnel:

- A run opens its live page at once, the WebSocket connects (`101 Switching Protocols`) and the events
  arrive over it, not through the polling fallback.
- The pipeline, the tiles and the three charts update live during the workload, the run is stored
  and the **View run** button appears when it ends.
- Refreshing the page in the middle of a run restores the whole state at the right step.
- Going offline for about five seconds and back online reconnects by itself and fills the charts in.
- Headers typed in the form reach the target: the same benchmark that failed every request without
  headers finished with none failed.

Not verified:

- A long silent stretch (a build of many minutes) against the tunnel's idle timeouts.
- A named Cloudflare tunnel, other browsers, phones, more than one viewer of a run.
- The polling fallback and the 4404 and 4429 close codes in a real browser (unit tests only).
