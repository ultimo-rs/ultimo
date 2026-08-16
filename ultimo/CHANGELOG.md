# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.0](https://github.com/ultimo-rs/ultimo/compare/ultimo-v0.8.0...ultimo-v0.9.0) - 2026-08-16

### Added

- typed Server-Sent Events (ctx.sse + SseEvent) ([#173](https://github.com/ultimo-rs/ultimo/pull/173))
- [**breaking**] streaming response bodies (UltimoBody + ctx.stream) ([#171](https://github.com/ultimo-rs/ultimo/pull/171))

## [0.8.0](https://github.com/ultimo-rs/ultimo/compare/ultimo-v0.7.0...ultimo-v0.8.0) - 2026-08-06

### Added

- *(oidc)* [**breaking**] verify OIDC/JWKS (RS256/ES256) tokens ([#169](https://github.com/ultimo-rs/ultimo/pull/169))

## [0.7.0](https://github.com/ultimo-rs/ultimo/compare/ultimo-v0.6.1...ultimo-v0.7.0) - 2026-08-06

### Added

- *(handler)* [**breaking**] typed request extractors (Path/Query/Json/Valid) ([#167](https://github.com/ultimo-rs/ultimo/pull/167))

## [0.6.1](https://github.com/ultimo-rs/ultimo/compare/ultimo-v0.6.0...ultimo-v0.6.1) - 2026-08-06

### Added

- *(rpc)* generate TanStack Query React hooks (client-gen) ([#164](https://github.com/ultimo-rs/ultimo/pull/164))

## [0.6.0](https://github.com/ultimo-rs/ultimo/compare/ultimo-v0.5.1...ultimo-v0.6.0) - 2026-07-08

### Added

- *(database)* [**breaking**] bump sqlx 0.7 -> 0.8 to fix RUSTSEC-2024-0363 ([#159](https://github.com/ultimo-rs/ultimo/pull/159))
- *(security)* add rate limiting middleware (token bucket) ([#154](https://github.com/ultimo-rs/ultimo/pull/154))
- *(core)* add serve_docs() for one-line interactive API documentation ([#153](https://github.com/ultimo-rs/ultimo/pull/153))
- *(security)* add IP allow/deny middleware with CIDR support ([#131](https://github.com/ultimo-rs/ultimo/pull/131))
- *(cli)* implement `ultimo dev` hot-reload dev server ([#130](https://github.com/ultimo-rs/ultimo/pull/130))

### Fixed

- *(deps)* bump jsonwebtoken 9 -> 10 to fix CVE-2026-25537 ([#158](https://github.com/ultimo-rs/ultimo/pull/158))
- address security review findings (session, rate-limit, error leaks, WebSocket CSWSH) ([#157](https://github.com/ultimo-rs/ultimo/pull/157))
