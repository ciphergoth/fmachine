#!/bin/sh -e

cp -v target/release/fmachine /usr/local/bin
adduser --system fmachine
adduser fmachine gpio
adduser fmachine input
cp -v etc/fmachine.conf /etc/
cp -v etc/fmachine.service /etc/systemd/system/
systemctl enable fmachine
