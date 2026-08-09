# RsGround

RsGround is a collaborative Rust playground with an Actix-Web backend, a
SolidJS frontend, and a WebAssembly operational-transform layer.

## Quick start

From the repository root:

```bash
wasm-pack build frontend-wasm --target web
pnpm --dir frontend install --frozen-lockfile
```

Start the API:

```bash
cargo run
```

It listens on `http://127.0.0.1:8080`. Set `RSGROUND_BIND` to use another
address. Guest login works without OAuth configuration; set the variables in
`.env.example` when GitHub login is needed. Always set a strong `JWT_SECRET`
outside local development.

In a second terminal, start the frontend:

```bash
pnpm --dir frontend dev
```

Then open the URL printed by Vite, normally `http://localhost:3000`.

Rust editor features are provided by Rust Analyzer over the backend WebSocket
bridge. Local development needs `rust-analyzer` and the `rust-src` component
in the active Rust toolchain; a deployed runner image must include both.

The vendored runner image is used when `backend/runner/lxc_rootfs` exists. A
fresh development checkout falls back to the host rootfs inside hakoniwa's
namespaces so the playground can start immediately; install the vendored
image before deploying.

## Deployment

Deploy the backend behind a TLS reverse proxy and use `wss://` for WebSockets.
For a non-loopback or production bind, startup requires:

- `RSGROUND_ENV=production`
- `JWT_SECRET` with at least 32 random bytes
- `RSGROUND_CORS_ORIGINS` containing the exact frontend origin(s)
- GitHub OAuth variables and a callback URL when GitHub login is enabled
- `backend/runner/lxc_rootfs` in the deployed artifact

Release binaries refuse the host-rootfs fallback and fail startup when the
vendored image is missing. The runner also requires a Linux kernel with
Landlock filesystem support. Set `RSGROUND_MAX_PROJECTS` to match the memory
and process budget of the host; it defaults to 64. `RSGROUND_MAX_USERS`
defaults to 10,000 to bound the in-memory guest registry.

Put request and WebSocket rate limiting, connection limits, and request-size
limits at the reverse proxy as well. The application limits active projects,
users, files, paths, document size, and command wall time, but it is
intentionally not a complete internet-facing rate limiter.

See [SECURITY.md](SECURITY.md) for the threat model and deployment checklist.

## Tests

Integration tests start an isolated API automatically:

```bash
cargo test --workspace --all-targets -- --test-threads=1
```

Use `RSGROUND_TEST_API_URL=http://127.0.0.1:PORT` to run them against an
already-running API instead.
