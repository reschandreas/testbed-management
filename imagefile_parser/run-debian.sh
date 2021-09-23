#!/usr/bin/env sh
echo "hello"
dd if=/dev/zero of=/var/dummy bs=8126k
shutdown -P now