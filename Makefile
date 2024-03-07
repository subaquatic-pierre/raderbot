.PHONY: run watch clean

run:
	cargo run

build:
	cargo build --release
	cp ./target/release/raderbot .
	chmod +x ./raderbot

dev:
	cargo watch -x "run"

fix:
	cargo fix --bin "raderbot"

test:
	cargo test

clean:
	cargo clean

rust-docs:
	cargo doc --no-deps

serve-rust-docs:
	python -m http.server --directory ./target/doc 3001

docs:
	scipts/build-docs.sh

serve-docs:
