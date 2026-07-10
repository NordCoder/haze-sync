# Reverse Proxy, TLS, and Public Access Boundary

## Scope

This DEP-P6 runbook documents a secure-by-default public access topology for Haze Sync Server and the tracked placeholder configuration in:

```text
deploy/reverse-proxy/Caddyfile
```

The example uses Caddy as the only public HTTP endpoint. It does not add a running proxy service to Compose, modify Server authentication, provision DNS, create certificates, open firewalls, or mutate a remote host.

## Selected topology

Production-style topology:

```text
public client
  -> TCP 443 on Caddy
  -> TLS termination on Caddy
  -> plaintext HTTP over host loopback
  -> 127.0.0.1:8080 Haze Sync Server
```

Optional TCP 80 may be open only for automatic HTTPS redirects and ACME HTTP challenges when the chosen certificate flow requires it.

The Server port must remain bound to loopback or otherwise blocked from untrusted networks. PostgreSQL must also remain loopback-only, private-network-only, or container-network-only.

## Placeholder host

The tracked Caddyfile uses:

```text
sync.example.com
```

Replace it with an operator-controlled hostname before starting Caddy. Do not run the placeholder unchanged as a production service.

Public DNS must already point to the intended host. DEP-P6 does not automate DNS records or cloud-provider configuration.

## TLS

The example relies on Caddy automatic HTTPS rather than tracked certificate paths. This keeps certificate and private-key material out of the repository.

Operator requirements:

- control the public hostname;
- ensure DNS resolves to the intended host;
- allow required inbound TCP 80/443 according to the certificate flow;
- allow required outbound DNS and certificate-authority traffic;
- keep Caddy state and private key material in its host-local protected storage;
- never copy certificate private keys into the repository, reports, CI artifacts, or general logs.

The tracked example does not contain certificate files, private keys, account credentials, or ACME secrets.

## Server authentication boundary

Caddy is transport and exposure infrastructure only. Server remains the authentication and authorization authority.

Current accepted route expectations:

| Route surface | Public proxy behavior | Server expectation |
| --- | --- | --- |
| `/health` | blocked with `404` by the tracked public listener | dependency-free process health; used by Caddy over loopback |
| `/ready` | blocked with `404` by the tracked public listener | sanitized dependency readiness; intended for trusted local/operator checks |
| `/v1/server-info` | proxied | public capability metadata |
| `/v1/files/**` | proxied | bearer authentication and role checks required |
| `/v1/changes` | proxied | bearer authentication and role checks required |
| `/v1/conflicts**` | proxied | bearer authentication and role checks required |
| `/v1/admin/**` | proxied | admin bearer authentication required |

Do not configure Caddy to inject a bearer token, replace the `Authorization` header, or bypass Server role checks. Standard reverse proxy behavior preserves the incoming authorization header for Server evaluation.

The public proxy may reject malformed transport requests, oversized bodies, or blocked health routes before Server sees them. Application authentication and stable API error semantics remain Server/API responsibilities.

## Health exposure

The tracked example hides both health endpoints from public clients:

```text
/health -> 404
/ready  -> 404
```

Caddy performs its active upstream check directly against loopback `/health`.

An operator may expose `/health` only when an external load balancer requires process-liveness checks and the exposure is intentionally accepted. `/ready` should remain private because it reveals dependency state even though its payload is sanitized.

For trusted local checks:

```bash
curl -fsS http://127.0.0.1:8080/health
curl -fsS http://127.0.0.1:8080/ready
```

Do not interpret either endpoint as proof that migrations, authentication credentials, provider adapters, backups, or full sync behavior are ready.

## Upload and request body limit

The accepted Server/API upload contract is:

```text
52,428,800 bytes
50 MiB
```

The tracked Caddyfile uses the exact byte value:

```text
request_body {
    max_size 52428800
}
```

Rules:

- do not lower the proxy limit without coordinating client-visible behavior;
- do not raise the proxy limit without a Server/API contract change;
- keep the Server-side validation as the application authority;
- expect Caddy to return HTTP `413` when the proxy limit is exceeded;
- do not log request bodies or uploaded file bytes.

The Caddy `request_body` directive requires Caddy 2.10 or newer. Pin and review the deployed Caddy version through the host package/image policy rather than silently relying on an unknown system version.

## Forwarded headers

The example does not add custom request-header rewriting. Caddy supplies standard reverse-proxy forwarding metadata and preserves application headers, including `Authorization`, unless explicitly configured otherwise.

Do not trust client-supplied forwarding headers from a path that can reach Server directly. The primary defense is to make Server unreachable from untrusted networks and accept public traffic only through the intended proxy.

Any future use of client IPs for authorization, rate limiting, or audit must define trusted-proxy behavior explicitly. DEP-P6 does not introduce that policy.

## Firewall and port boundary

Recommended inbound boundary:

| Port | Exposure | Purpose |
| --- | --- | --- |
| TCP 443 | public | HTTPS API through Caddy |
| TCP 80 | optional public | HTTPS redirect and ACME HTTP challenge only |
| TCP 8080 | not public | Haze Sync Server loopback upstream |
| TCP 5432 | not public | PostgreSQL local/private dependency |
| TCP 2019 | not public | Caddy admin API; retain loopback/default local boundary |

Apply equivalent rules for both IPv4 and IPv6. A loopback bind is preferable to relying only on a firewall rule for Server and database ports.

Do not expose Docker-published Server or PostgreSQL ports on `0.0.0.0` or `[::]` for this topology.

## Configuration validation

Use Caddy 2.10 or newer.

Check formatting:

```bash
caddy fmt --diff deploy/reverse-proxy/Caddyfile
```

Validate the adapted configuration without starting it:

```bash
caddy validate \
  --config deploy/reverse-proxy/Caddyfile \
  --adapter caddyfile
```

The example uses automatic HTTPS and therefore does not require checked-in certificate files for validation. Successful validation proves configuration loading/provisioning only. It does not prove:

- DNS correctness;
- certificate authority reachability;
- successful certificate issuance;
- firewall correctness;
- Server reachability;
- authentication correctness;
- upload success;
- production readiness.

Before an authorized rollout, replace the placeholder hostname and repeat validation on the target host.

## Post-start smoke checks

Use a non-secret public hostname variable:

```bash
export HAZE_SYNC_PUBLIC_HOST=sync.example.com
```

Check HTTPS and the intentionally public server-info surface:

```bash
curl -fsS "https://${HAZE_SYNC_PUBLIC_HOST}/v1/server-info"
```

Confirm public health endpoints are hidden:

```bash
curl -sS -o /dev/null -w '%{http_code}\n' \
  "https://${HAZE_SYNC_PUBLIC_HOST}/health"
curl -sS -o /dev/null -w '%{http_code}\n' \
  "https://${HAZE_SYNC_PUBLIC_HOST}/ready"
```

Expected tracked-example status for both is `404`.

For authenticated routes, use an operator-local secret source and do not paste the token into shell history, reports, screenshots, CI logs, or committed scripts. DEP-P6 does not provide a token bootstrap or rotation procedure.

## Logging and secrecy

Proxy logs must not contain:

```text
Authorization values
bearer tokens or token hashes
Idempotency-Key values
request bodies or uploaded bytes
OAuth tokens
production database URLs
TLS private keys
raw provider payloads
vault contents
```

The tracked example intentionally does not enable an access-log format. A later operations phase may add sanitized log and retention policy.

## Non-goals preserved

DEP-P6 does not:

```text
add Caddy to Docker Compose
start or reload a proxy
open firewall ports
provision DNS records
commit certificate or private-key material
change Server/API authentication
change the 52,428,800-byte upload contract
inject bearer credentials
add cloud-provider automation
add remote-host automation
claim production readiness
```
