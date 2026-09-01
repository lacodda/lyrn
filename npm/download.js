// Downloads the lyrn binary matching this package version from GitHub Releases.
// Shared by install.js (postinstall, where lifecycle scripts are allowed) and
// run.js (first-run fallback: npm v12 skips install scripts by default).
const fs = require("fs");
const https = require("https");
const path = require("path");
const { spawnSync } = require("child_process");

const pkg = require("./package.json");
const REPO = "lacodda/lyrn";
// The wrapper can be patched independently of the Rust binary: an explicit
// lyrn.binary field pins the release tag, otherwise it follows the version.
const TAG = (pkg.lyrn && pkg.lyrn.binary) || `v${pkg.version}`;

const TARGETS = {
  "win32-x64": ["x86_64-pc-windows-msvc", "zip"],
  "linux-x64": ["x86_64-unknown-linux-gnu", "tar.gz"],
  "darwin-arm64": ["aarch64-apple-darwin", "tar.gz"],
};

const exe = process.platform === "win32" ? "lyrn.exe" : "lyrn";
const exePath = path.join(__dirname, exe);

function download(url, file, redirects, done) {
  if (redirects > 5) return done(new Error("too many redirects"));
  https
    .get(url, { headers: { "user-agent": "lyrn-npm" } }, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        res.resume();
        return download(res.headers.location, file, redirects + 1, done);
      }
      if (res.statusCode !== 200) {
        res.resume();
        return done(new Error(`HTTP ${res.statusCode} for ${url}`));
      }
      const out = fs.createWriteStream(file);
      res.pipe(out);
      out.on("finish", () => out.close(done));
      out.on("error", done);
    })
    .on("error", done);
}

function install(done) {
  const key = `${process.platform}-${process.arch}`;
  const entry = TARGETS[key];
  if (!entry) {
    console.error(`lyrn: no prebuilt binary for ${key}; install with: cargo install lyrn`);
    process.exit(1);
  }
  const [target, ext] = entry;
  const name = `lyrn-${TAG}-${target}`;
  const url = `https://github.com/${REPO}/releases/download/${TAG}/${name}.${ext}`;
  const archive = path.join(__dirname, `archive.${ext}`);

  console.log(`lyrn: downloading ${url}`);
  download(url, archive, 0, (err) => {
    if (err) {
      console.error(`lyrn: download failed: ${err.message}`);
      process.exit(1);
    }
    const result =
      ext === "zip"
        ? spawnSync(
            "powershell.exe",
            ["-NoProfile", "-NonInteractive", "-Command", "Expand-Archive -LiteralPath 'archive.zip' -DestinationPath . -Force"],
            { cwd: __dirname, stdio: "inherit" },
          )
        : spawnSync("tar", ["-xzf", `archive.${ext}`], { cwd: __dirname, stdio: "inherit" });
    if (result.status !== 0) {
      console.error("lyrn: cannot extract the archive");
      process.exit(1);
    }
    fs.renameSync(path.join(__dirname, name, exe), exePath);
    fs.rmSync(path.join(__dirname, name), { recursive: true, force: true });
    fs.rmSync(archive, { force: true });
    if (process.platform !== "win32") {
      fs.chmodSync(exePath, 0o755);
    }
    console.log(`lyrn: installed lyrn ${TAG}`);
    done();
  });
}

module.exports = { install, exePath };
