.PHONY: run watch clean

run:
	cargo run

build:
	cargo build --release
	cp ./target/release/raderbot .
	chmod +x ./raderbot

watch:
	cargo watch -x "run"

fix:
	cargo fix --bin "raderbot"

test:
	cargo test

clean:
	cargo clean