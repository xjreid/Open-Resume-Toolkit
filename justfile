set dotenv-load := false

default:
	@just --list

bootstrap:
	@node tools/bootstrap.mjs
	pnpm install --frozen-lockfile

generate:
	pnpm generate

check:
	pnpm check
	cargo fmt --all --check
	cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
	cargo test --workspace --all-targets --locked

dev:
	@node tools/assert-dev-profile.mjs
	pnpm dev

dev-extension browser:
	pnpm "dev:extension:{{browser}}"

test-integration:
	cargo test --workspace --locked
	pnpm test

test-platform:
	@echo "M1 native vault suites are intentionally gated pending signed macOS and Windows VM harnesses."

# Synthetic App Sandbox measurements only; never enables the importer.
probe-document-sandbox-macos:
	node tools/probe-document-sandbox-macos.mjs

# Separate synthetic XPC supervisor/direct-child lifecycle experiment.
probe-document-lifecycle-macos:
	node tools/probe-document-lifecycle-macos.mjs

test-platform-vault:
	ORT_RUN_OS_VAULT_TESTS=1 cargo test -p ort-vault --test os_vault native_database_key_round_trip_and_overwrite_denial -- --ignored --exact --nocapture

package-preview:
	@node tools/assert-dev-profile.mjs
	pnpm --filter @ort/desktop tauri build --config src-tauri/tauri.preview.conf.json

verify-artifacts:
	@echo "Artifact verification becomes active with the M7 packaging pipeline."
