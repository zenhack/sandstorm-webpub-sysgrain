all: webpub
dev: all
	spk dev
clean:
	rm -rf webpub target/

# TODO: use release mode or parametrize at some point:
webpub: $(PWD)/target/debug/webpub
	cp $< $@

$(PWD)/target/debug/webpub: Cargo.toml
	cargo build

-include $(wildcard target/*/*.d)

.PHONY: all dev clean
