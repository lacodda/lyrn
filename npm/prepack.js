// Copies the root README into the package before it is packed.
//
// The npm page must show the same text as GitHub and crates.io, and npm only
// publishes what sits next to package.json. Doing this in the publish workflow
// instead would mean a manual `npm publish` ships a page with no README at
// all; `prepack` runs for every pack, CI or hand.
const fs = require("fs");
const path = require("path");

const source = path.join(__dirname, "..", "README.md");
const target = path.join(__dirname, "README.md");

fs.copyFileSync(source, target);
console.log("lyrn: README.md copied from the repository root");
