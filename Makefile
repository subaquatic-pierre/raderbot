.PHONY: run watch clean

run:
	cargo run

watch:
	cargo watch -x "run"

clean:
	cargo clean