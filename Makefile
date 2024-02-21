.PHONY: run watch clean

run:
	cargo run

watch:
	cargo watch -x "run"

test:
	cargo test

clean:
	cargo clean