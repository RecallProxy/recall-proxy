# E2E Local Validation

Docker Compose-based end-to-end validation for RecallProxy. The first scenario exercises the gateway HTTP surface with a mock hindsight backend so the flow runs without cloud credentials.

## Prerequisites

- Docker and Docker Compose (v2)
- [promptfoo](https://www.promptfoo.dev/) (`npm install -g promptfoo`)

## Quick Start

### 1. Start the stack

```bash
docker compose -f examples/e2e/docker-compose.yml up --wait
```

`--wait` blocks until both the gateway and mock-hindsight services report healthy. The gateway binds to `0.0.0.0:8080` and the mock hindsight service to `0.0.0.0:8081`.

### 2. Run promptfoo tests

```bash
cd examples/e2e
promptfoo eval
```

This exercises both the `/health` and `/ingest` endpoints and asserts:

- Health returns HTTP 200 with `status: "ok"` and `service: "recall-proxy-gateway"`.
- Ingest returns HTTP 200 with `status: "stored"`.

### 3. Tear down

```bash
docker compose -f examples/e2e/docker-compose.yml down
```

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `RECALL_PROXY_BIND_ADDRESS` | `0.0.0.0:8080` | Bind address for the gateway HTTP server |
| `RUST_LOG` | `info` | Structured log level (e.g. `debug`, `trace`) |
| `RUST_LOG_FORMAT` | `text` | Log format: `text` or `json` |

Override by editing `examples/e2e/docker-compose.yml` or passing `-e` to `docker compose run`.

## Architecture Notes

- **Gateway service** builds from the repository root `Dockerfile` (multi-stage Rust build).
- **mock-hindsight service** runs a Python HTTP server that simulates the hindsight backend at `/hindsight/ingest`.
- The gateway's `/ingest` handler returns a deterministic `{"status":"stored"}` response, so no real backend is required for this first pass.
- The mock hindsight service is reachable from the gateway at `http://mock-hindsight:8081` via the Docker network.

## CI Integration

The flow is designed for easy CI reuse:

1. `docker compose -f examples/e2e/docker-compose.yml up --wait` starts the environment.
2. `promptfoo eval` runs the assertions.
3. `docker compose -f examples/e2e/docker-compose.yml down` cleans up.

No additional configuration is needed — all services and assertions are self-contained in this directory.

## Troubleshooting

- **Healthcheck fails to pass**: Ensure port `8080` is free on the host. The gateway binds to `0.0.0.0:8080` by default.
- **promptfoo `is-json` assertion fails**: The gateway returns valid JSON on all paths. If the health endpoint returns an error page (e.g., from a reverse proxy), verify the gateway is reachable at `http://localhost:8080/health`.
- **Mock hindsight not reachable**: The gateway expects `MOCK_HINDSIGHT_URL=http://mock-hindsight:8081`. Do not change the service name `mock-hindsight` in `docker-compose.yml` without also updating the gateway's environment variable.
