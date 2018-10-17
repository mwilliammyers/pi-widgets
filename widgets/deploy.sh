#!/usr/bin/env bash

# host=10.37.246.192
host=10.37.247.57
target=arm-unknown-linux-musleabi

cross build --target $target --release && \
	scp ./target/$target/release/widgets pi@$host:/tmp && \
	ssh -t pi@$host "sudo mv /tmp/widgets /usr/local/bin && sudo systemctl restart widgets"
