all: webpub
dev: all
	spk dev

# TODO: use release mode or parametrize at some point:
webpub: target/debug/webpub
	cp $< $@

target/debug/webpub:
	cargo build

-include target/*/*.d

.PHONY: all dev
