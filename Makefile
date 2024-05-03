.PHONY: run
run: build
	sudo target/debug/virtual-usb

.PHONY: build
build:
	cargo build
