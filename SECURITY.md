# Security

RsGround executes user-supplied Rust code. Treat a public deployment as an
untrusted-code service, not as a normal web application.

## Hardening in this branch

- Jobs use user, PID, mount, and network namespaces. The network namespace is
  isolated, and the root filesystem is read-only except for the per-project
  home and temporary files.
- Landlock restricts job filesystem access to the toolchain, system read-only
  paths, `/home`, `/tmp`, `/dev`, and `/proc`.
- CPU, address-space, file-size, process, file-descriptor, and wall-clock
  limits are applied to runner commands.
- A missing runner image can use the host rootfs only for debug builds. Release
  and production startup reject that fallback.
- Project passwords are stored as Argon2 hashes and are not returned by the
  API. Password submission uses `X-Project-Password` instead of a URL query.
- Deployment requires an explicit JWT secret and CORS allowlist. OAuth state is
  bound to an HttpOnly cookie and checked on callback.
- Active projects, project files, file paths, document size, and WebSocket
  continuation size are bounded.
- The repository's application code contains no `unsafe` blocks. The previous
  local unsafe reader implementation was replaced with async-io's safe API.

## Deployment checklist

1. Provide `RSGROUND_ENV=production`, a randomly generated `JWT_SECRET` of at
   least 32 bytes, and exact `RSGROUND_CORS_ORIGINS` values.
2. Include `backend/runner/lxc_rootfs` in the release artifact. Do not run a
   release binary with the development fallback.
3. Run the service as a dedicated unprivileged account. Verify that the host
   permits the namespaces and Landlock features required by hakoniwa.
4. Terminate TLS before the frontend and backend. Do not expose the backend's
   plain HTTP listener directly to the internet.
5. Configure the reverse proxy with authentication-independent rate limits,
   connection limits, WebSocket timeouts, and access-log redaction for
   `Authorization`, `Sec-WebSocket-Protocol`, and `X-Project-Password`.
6. Set `RSGROUND_MAX_PROJECTS` conservatively and monitor CPU, memory, disk,
   process count, and `/tmp` usage.

## Known limitations

The application is currently in-memory: JWTs cannot be revoked and projects
are lost on restart. There is no distributed rate limiter, persistent user
store, or automatic project cleanup. A public deployment must provide those
controls at the proxy or platform layer.

`cargo audit` reports one transitive unmaintained crate, `atomic-polyfill`,
through `hakoniwa -> postcard -> heapless`; it has no reported vulnerability or
available replacement in the current hakoniwa dependency chain.

Please report security issues privately through the repository's supported
GitHub security-reporting channel rather than opening a public issue with an
exploitable proof of concept.
