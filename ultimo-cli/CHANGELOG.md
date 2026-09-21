# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.2](https://github.com/ultimo-rs/ultimo/compare/ultimo-cli-v0.9.1...ultimo-cli-v0.9.2) - 2026-09-21

### Added

- *(app)* add Ultimo::mount_rpc() to mount an RpcRegistry in one call ([#188](https://github.com/ultimo-rs/ultimo/pull/188))

### Fixed

- *(middleware)* logger() is a silent no-op without a tracing subscriber ([#187](https://github.com/ultimo-rs/ultimo/pull/187))
- *(rpc)* bare Vec<T>/Option<T> RPC types drop TS decls; fullstack template now uses RpcRegistry ([#184](https://github.com/ultimo-rs/ultimo/pull/184))

## [0.9.1](https://github.com/ultimo-rs/ultimo/compare/ultimo-cli-v0.9.0...ultimo-cli-v0.9.1) - 2026-09-01

### Added

- *(cli)* ultimo mcp server for AI coding agents ([#179](https://github.com/ultimo-rs/ultimo/pull/179))
- *(cli)* agent-proof scaffolding ([#176](https://github.com/ultimo-rs/ultimo/pull/176))

## [0.6.0](https://github.com/ultimo-rs/ultimo/compare/ultimo-cli-v0.5.1...ultimo-cli-v0.6.0) - 2026-07-08

### Added

- *(cli)* implement `ultimo generate --watch` + scaffold generate-client in templates ([#155](https://github.com/ultimo-rs/ultimo/pull/155))
- *(cli)* implement `ultimo dev` hot-reload dev server ([#130](https://github.com/ultimo-rs/ultimo/pull/130))
