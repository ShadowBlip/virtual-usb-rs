.PHONY: run
run:
	CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER='sudo -E' cargo run --example steam_deck

.PHONY: build
build:
	cargo build
