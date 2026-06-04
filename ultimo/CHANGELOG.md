# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.2](https://github.com/ultimo-rs/ultimo/compare/ultimo-v0.2.1...ultimo-v0.2.2) - 2026-06-04

### Added

- session management (cookies + secure sessions, in-memory store) — closes #3 ([#43](https://github.com/ultimo-rs/ultimo/pull/43))
- testing utilities (TestClient, assertions, middleware/DB helpers, fixtures) — closes #4 ([#37](https://github.com/ultimo-rs/ultimo/pull/37))

### Fixed

- *(router)* static routes beat params regardless of order; add raw_body (#21, #22) ([#45](https://github.com/ultimo-rs/ultimo/pull/45))
