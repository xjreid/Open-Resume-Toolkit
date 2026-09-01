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

package-preview:
	@node tools/assert-dev-profile.mjs
	pnpm --filter @ort/desktop tauri build

verify-artifacts:
	@echo "Artifact verification becomes active with the M7 packaging pipeline."
