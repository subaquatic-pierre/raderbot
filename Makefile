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

docs:
	cargo doc --no-deps

serve-docs:
	python -m http.server --directory ./target/doc 3001