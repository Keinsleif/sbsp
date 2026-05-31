#!/usr/bin/env node
const fs = require("fs");

const pnpm_data = JSON.parse(fs.readFileSync("./tmp.json", "utf-8"));

let md = `## Node.js Dependencies

`;

for (const i in pnpm_data) {
    let license_text;
    if (pnpm_data[i].licenseFile) {
        license_text = fs.readFileSync(pnpm_data[i].licenseFile, 'utf-8');
    } else {
        license_text = 'no license provided';
    }
    md += `### [${i}](${pnpm_data[i].repository})
license: ${pnpm_data[i].licenses}

\`\`\`text
${license_text}
\`\`\`

`;
}

fs.appendFileSync("../THIRD_PARTY_NOTICES.md", md);
console.log("Generated THIRD_PARTY_NOTICES.md");
