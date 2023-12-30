# Name for the output iso file
ISO_NAME := bootable.iso
# Path to where all system files will be placed in
BUILD_DIR := ./build
# Path to the video
VIDEO := ./input.mp4
# In seconds
VIDEO_LEN := 60

.PHONY: build run clean


NO_ENCODE := 0
ENCODE? := encode_video
ifeq ($(NO_ENCODE), 1)
    ENCODE? =
endif


# Build an iso file
build: mkdir $(ENCODE?) compile
	genisoimage -o $(ISO_NAME) -b EFI/BOOT/BOOTX64.EFI -no-emul-boot -iso-level 3 -udf -r -J $(BUILD_DIR)

# Create the build directory
mkdir:
	mkdir -p $(BUILD_DIR)/EFI/BOOT/

# Encode the video
encode_video: mkdir
	ffmpeg -i $(VIDEO) -vf "scale=384:216" -t $(VIDEO_LEN) -f rawvideo -pix_fmt yuv420p $(BUILD_DIR)/VIDEO_BYTES -y

# Compile the executable
compile: mkdir
	cargo build --target x86_64-unknown-uefi --release
	cp ./target/x86_64-unknown-uefi/release/rick_os.efi $(BUILD_DIR)/EFI/BOOT/BOOTX64.EFI

# Run the iso on a vm
run: build
	qemu-system-x86_64 -enable-kvm \
	-drive if=pflash,format=raw,readonly=on,file=OVMF_CODE.fd \
	-drive if=pflash,format=raw,readonly=on,file=OVMF_VARS.fd \
	-cdrom $(ISO_NAME)

# Clear all build files
clean:
	rm -rf $(BUILD_DIR)
	cargo clean
	rm -f $(ISO_NAME)
