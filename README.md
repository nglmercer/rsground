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

The vendored runner image is used when `backend/runner/lxc_rootfs` exists. A
fresh development checkout falls back to the host rootfs inside hakoniwa's
namespaces so the playground can start immediately; install the vendored
image before deploying.

## Tests

The integration tests expect a running API. Start it first, then run:

```bash
cargo test --workspace --all-targets -- --test-threads=1
```

Use `RSGROUND_TEST_API_URL=http://127.0.0.1:PORT` when the API is listening on
a non-default address.
