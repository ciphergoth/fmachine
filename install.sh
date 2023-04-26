#!/bin/sh -e

if [ "$(id -u)" -ne 0 ]; then
    echo "Not root, re-execing as root"
    exec sudo /bin/sh -e $0 "$@"
fi

cp -v target/release/fmachine /usr/local/bin
adduser --system fmachine
adduser fmachine gpio
adduser fmachine input
cp -v etc/fmachine.service /etc/systemd/system/
systemctl enable fmachine
