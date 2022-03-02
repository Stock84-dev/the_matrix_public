ifndef p
$(error p is not set)
endif

ifndef m
$(error p is not set)
endif

ifndef bump
$(error bump is not set)
endif

release:
	# increase version (in Cargo.toml and clap)
	cargo build --release -p the_architect
	$(eval version := $(shell ./target/release/the_architect bump -p $(p) -k $(bump)))
	# test
	cargo test --all-features -p $(p) -- --ignored
	# build
	cargo build --release -p $(p)
	# copy to release
	if [ ! -d "release" ]; then mkdir "release"; fi
	if [ ! -d "release/$(p)" ]; then mkdir release/$(p); fi
	rm release/$(p)/* || exit 0
	cp target/release/$(p) release/$(p)/$(p)-$(version)
	# push
	git add ${p}/*
	git commit -m "Bump '$(p)' to $(version)."
	git push

.PHONY: release push

# to cross compile use this command
# PKG_CONFIG_ALLOW_CROSS=true PKG_CONFIG_ALL_STATIC=true PKG_CONFIG_PATH=/lib/pkgconfig PG_CONFIG_X86_64_UNKNOWN_LINUX_GNU=/usr/bin/pg_config cb -p matrix --release --target x86_64-unknown-linux-musl
