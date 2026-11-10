#!/bin/sh
export RUSTUP_HOME=/Volumes/Samsung-SSD/rust/.rustup
export CARGO_HOME=/Volumes/Samsung-SSD/rust/.cargo
export PATH="$CARGO_HOME/bin:$PATH"
cd /Volumes/Samsung-SSD/developer/VSCodeProjects/ohmic || exit 1
cargo fetch > /tmp/ohmic-fetch.log 2>&1
echo "fetch_exit=$?" >> /tmp/ohmic-fetch.log
cargo check > /tmp/ohmic-check.log 2>&1
echo "check_exit=$?" >> /tmp/ohmic-check.log
echo done > /tmp/ohmic-build.done
