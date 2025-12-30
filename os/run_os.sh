#!/bin/bash
qemu-system-x86_64 -m 1024 -kernel kernel/bzImage -initrd initramfs.cpio -append "console=ttyS0,115200 console=tty0 init=/init" -display curses
