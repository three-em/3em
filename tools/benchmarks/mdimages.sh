imgupload() {
  curl 'https://ipfs.infura.io:5001/api/v0/add?pin=true&cid-version=1' \
    -F "path=@$1" \
    --compressed --silent |
    jq -r '.Hash'
}

echo "\n<details><summary>Graphs</summary>\n"
for file in "$@"; do
  echo "![$(basename $file)](https://images.weserv.nl/?url=ipfs.infura.io/ipfs/$(imgupload $file))"
done
echo "\n</details>\n"
