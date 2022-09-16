TARGET_URL ?= blindspi.local
TARGET_HOST ?= pi@$(TARGET_URL)
REMOTE_DIRECTORY ?= /home/pi
ARM_BUILD_PATH ?= target/debian/blinds_*.deb
VERSION_TAG = $(shell cargo get version)
MENDER_ARTIFACT_NAME ?= blinds-$(VERSION_TAG)
MENDER_ARTIFACT_OUTPUT_PATH ?= $(MENDER_ARTIFACT_NAME).mender
MENDER_DEVICE_TYPE ?= raspberrypi4


.PHONY: build
build:
	cargo build --release
	cargo deb --no-build

.PHONY: install
install: build
	sudo dpkg -i $(ARM_BUILD_PATH)

.PHONY: deploy
deploy: build
	@echo "Sending $(ARM_BUILD_PATH) to $(TARGET_HOST):$(REMOTE_DIRECTORY)"
	rsync -avz --delete $(ARM_BUILD_PATH) $(TARGET_HOST):$(REMOTE_DIRECTORY)

.PHONY: debug
debug:
	cargo run -- --config dev_config/config.yaml

.PHONY: build-artifact
build-artifact: build
	mkdir -p mender_target
	mender-artifact write module-image -T deb -n $(MENDER_ARTIFACT_NAME) -t $(MENDER_DEVICE_TYPE) -o mender_target/$(MENDER_ARTIFACT_OUTPUT_PATH) -f $(ARM_BUILD_PATH)

.PHONY: install-dependencies
install-dependencies:
	cargo install cargo-deb cargo-get
