#!/bin/sh -e

echo "This script is completely untested. Some dependencies are likely missing."
echo "Installing system dependencies with apt"
echo "Running sudo, you should be prompted for a password"
sudo apt install python3
echo "Installing Rust from https://rustup.rs/ with curl https://sh.rustup.rs -sSf | sh"
curl https://sh.rustup.rs -sSf | sh
echo "Updating pip"
python3 -m pip install --user --upgrade pip
echo "Installing dependencies for gen-svg.py"
python3 -m pip install --user -r plans/requirements.txt
