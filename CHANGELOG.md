# Changelog

## 🎉 [1.0.1] - 2023-09-03

### 🛠️ Bug Fixes

- The version of lyrn in the package.json has been changed to 1.0.0

## 🎉 [1.0.0] - 2023-09-02

### ✨ Features

- 📦 added npm packages installer
- Added ui generator for react
- Added webpack dev server launcher
- Added .gitignore, postcss.config.js and index.d.ts generators
- Added readme file generator
- Added some template files
- Added simple app file generator
- Added .eslintrc generator
- Added license file generator
- Added tsconfig generator
- Added helpers
- Added react template
- Added templates
- Added libs
- Added create_package function
- 📂 command directory added
- [**breaking**] 📦✨ next lyrn version started

### 🎛️ Refactor

- File generation improved
- Tsconfig struct replaced with json_serde::Value
- Eslintrc struct replaced with json_serde::Value
- HashMap in Package struct replaced with json_serde::Value

### 📖 Documentation

- The description of the application's features has been improved in the README file

### 🛟 Miscellaneous Tasks

- Fix github action "release"
- Add github action "release" and cliff.toml

### 🛠️ Bug Fixes

- Fix clear_console function for windows
- Fixed function clear_console
- Added sorting list of dependencies

