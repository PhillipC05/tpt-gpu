# SPDX-License-Identifier: Apache-2.0
# Out-of-tree build for the TPT GPU DRM module.
#
# Usage:
#   make KDIR=/lib/modules/$(uname -r)/build        # host kernel
#   make KDIR=/path/to/kernel/source ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu-
#
# The kernel must be configured with:
#   CONFIG_DRM=m (or y)
#   CONFIG_RUST=y
#   CONFIG_DRM_TPT_GPU=m

KDIR ?= /lib/modules/$(shell uname -r)/build
PWD  := $(shell pwd)

default:
	$(MAKE) -C $(KDIR) M=$(PWD) modules

install:
	$(MAKE) -C $(KDIR) M=$(PWD) modules_install
	depmod -a

clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean

.PHONY: default install clean
