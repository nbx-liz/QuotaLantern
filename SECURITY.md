# Security Policy

QuotaLantern's core promise is that it does not persist usage data, does not acquire conversation content or credentials, and does not send data to any server of its own. A finding that breaks any of these is a security issue, even if it looks minor.

## Reporting a vulnerability

Please report privately through GitHub: **Security → Report a vulnerability** on this repository (private vulnerability reporting). Do not open a public issue for a suspected vulnerability.

Include, where possible:

- QuotaLantern version or commit, OS version and CPU
- the affected component (adapter, IPC, core, UI, installer)
- reproduction steps using **synthetic data only**

Do not attach real conversations, credentials, account identifiers, file paths, raw provider output, or unredacted screenshots of official dashboards.

## Scope

In scope: anything in this repository, including persistence of usage data, leakage of identifiers past the adapter boundary, unexpected network traffic from the core or UI, arbitrary RPC / shell / file access from the UI, and local IPC connections from other users or remote hosts.

Out of scope for product guarantees (but still worth telling us about): OS swap and crash dumps, screen capture, storage performed by the official AI clients themselves, and already-compromised machines. See [`docs/security/guarantee-boundary.md`](docs/security/guarantee-boundary.md).

## Supported versions

QuotaLantern has not been released yet. Until the first release, only the `main` branch is supported.

---

脆弱性の報告は、日本語でも受け付けます。
