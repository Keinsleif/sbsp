#!/bin/sh

cd "$(dirname "$0")"
cd ..

cargo about generate about.hbs -o THIRD_PARTY_NOTICES.md

cd sbsp_frontend

npx license-checker --json > "$(dirname "$0")/tmp.json"
cd "$(dirname "$0")"
node notice_json2md.js
rm tmp.json