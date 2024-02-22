.PHONY: run watch clean

run:
	cargo run

build:
	cargo build --release

watch:
	cargo watch -x "run"

fix:
	cargo fix --bin "raderbot"

test:
	cargo test

clean:
	cargo clean