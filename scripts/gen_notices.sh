#!/bin/sh

cd "$(dirname "$0")"
cd ..

cargo about generate about.hbs -o THIRD_PARTY_NOTICES.md

cd sbsp_frontend

pnpm exec license-checker-rseidelsohn --excludePackages sbsp_frontend --direct false --production --json --out ../scripts/tmp.json
cd ../scripts
node notice_json2md.js
rm tmp.json
