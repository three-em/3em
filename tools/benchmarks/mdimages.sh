#!/bin/bash

imgupload() {
  curl 'https://ipfs.infura.io:5001/api/v0/add?pin=true&cid-version=1' \
    -F "path=@$1" --compressed --silent |
    jq -r '.Hash'
}

echo -e "\n<details><summary>Graphs</summary>\n"
for file in "$@"; do
  echo "![$(basename $file)](https://images.weserv.nl/?url=gateway.ipfs.io/ipfs/$(imgupload $file))"
done
echo -e "\n</details>\n"
