generate-protocols:
	scripts/gen-proto

build:
	cargo build

build-with-checker:
	cargo build --features "checker"

build-with-stats:
	cargo build --features "statistics"

build-all:
	cargo build --features "all"

run: build
	RUST_LOG=debug target/debug/risq daemon

regtest: build
	RUST_LOG=debug target/debug/risq daemon -n BtcRegtest --tor-active=false

check:
	cargo watch -x check

test:
	RUST_BACKTRACE=full cargo watch -s 'cargo test --features "all" -- --nocapture'

test-in-ci:
	cargo test --all-features --verbose --locked

build-minimal-release:
	cargo build --locked --release --no-default-features --features "fail-on-warnings"

build-arm-unknown-linux-gnueabihf-release:
	cargo build --locked --release --all-features --target arm-unknown-linux-gnueabihf

build-x86_64-unknown-linux-gnu-release:
	cargo build --locked --release --all-features --target x86_64-unknown-linux-gnu

run-tor:
	scripts/run-tor

no-of-deps:
	cargo tree | grep -v '(*)' | grep -v '\[' | wc -l
