# ADR 0001: Tauri 2, React, and Rust desktop foundation

- Status: accepted
- Milestone: M0

## Decision

Use one Tauri 2 desktop shell, React/TypeScript/Vite frontend, and Rust backend for macOS and Windows. Platform code remains behind narrow adapters. UI webviews receive no Node, shell, arbitrary filesystem, opener, or HTTP authority.

## Consequences

Both WebView2 and WKWebView require testing. The initial main and overlay windows expose only the typed health command and use bundled assets with a restrictive CSP.
