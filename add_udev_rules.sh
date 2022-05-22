#!/usr/bin/env bash

readonly udev_path="/etc/udev/rules.d/40-blinds.rules"

echo "writing udev rules to $udev_path"

cat <<EOT | sudo tee $udev_path > /dev/null

# CH340
KERNEL=="ttyUSB*", ATTRS{idVendor}=="1a86", ATTRS{idProduct}=="7523", MODE:="0777", SYMLINK+="blinds_motors"

EOT

sudo udevadm control --reload-rules && sudo udevadm trigger
echo "Done"
