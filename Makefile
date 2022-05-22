TARGET_URL ?= blindspi.local
TARGET_HOST ?= pi@$(TARGET_URL)
REMOTE_DIRECTORY ?= /home/pi
TARGET_ARCH ?= armv7-unknown-linux-musleabihf
ARM_BUILD_PATH ?= target/armv7-unknown-linux-musleabihf/debian/blinds_*_armhf.deb


.PHONY: build
build:
	cargo build --release --target=$(TARGET_ARCH)
	cargo deb --target=$(TARGET_ARCH) --no-build

.PHONY: deploy
deploy: build
	@echo "Sending $(ARM_BUILD_PATH) to $(TARGET_HOST):$(REMOTE_DIRECTORY)"
	rsync -avz --delete $(ARM_BUILD_PATH) $(TARGET_HOST):$(REMOTE_DIRECTORY)


.PHONY: debug
debug:
	cargo run -- --config dev_config/config.yaml
