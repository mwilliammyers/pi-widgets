#!/usr/bin/env bash

# host=10.37.246.192
# host=10.37.247.57
host=${1:-elderberry.local}
target=${2:-arm-unknown-linux-musleabi}
name=$(cargo metadata --no-deps --format-version 1 | perl -n -e '/name":\s*"(.*?)"/ && print $1')

cargo build --target $target --release && \
	scp ./target/$target/release/$name pi@$host:/tmp && \
	echo "DONE"
	# ssh -t pi@$host "sudo mv /tmp/$name /usr/local/bin && sudo systemctl restart $name"
