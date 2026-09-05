set dotenv-load := false

default:
	@just --list

bootstrap:
	@node tools/bootstrap.mjs
	pnpm install --frozen-lockfile

generate:
	pnpm generate

verify-contracts:
	cargo run --locked -p ort-contract-generator
	git diff --exit-code -- packages/contracts/generated

check:
	pnpm check
	just verify-contracts
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
	node tools/qualify-macos.mjs preflight
	cargo test --locked -p ort-desktop development_storage_rejects_other_package_identities
	cargo test --locked -p ort-storage profile_channel_mismatch_preserves_database_manifest_and_key
	@echo "M0 preflight and synthetic isolation passed. Run package-qualification and verify-macos-app; native M1 vault qualification remains separate."

# Local signing only; never exports a key or changes certificate trust.
package-qualification identity="ORT Local Test Signing":
	node tools/qualify-macos.mjs build "{{identity}}"

verify-macos-app app:
	node tools/qualify-macos.mjs verify "{{app}}"

# Requires a committed, clean source tree; retains its disposable checkout/logs.
qualify-clean-checkout:
	node tools/qualify-macos.mjs clean-checkout

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
