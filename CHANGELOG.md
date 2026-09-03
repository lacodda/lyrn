# Changelog

## 🎉 [2.4.0] - 2026-09-03

### ✨ Features

- Add the workspace form

## 🎉 [2.3.0] - 2026-09-02

### ✨ Features

- Add the service form, with the web UI as an add-on

### 📖 Documentation

- Changelog for 2.3.0

## 🎉 [2.2.0] - 2026-09-02

### ✨ Features

- Add the desktop form, with i18n as an add-on

### 📖 Documentation

- Changelog for 2.2.0

### 🛟 Miscellaneous Tasks

- Skip publishing a version that is already in the registry

## 🎉 [2.1.1] - 2026-09-01

### 📖 Documentation

- Changelog for 2.1.1

### 🛠️ Bug Fixes

- Keep the template files inside the published crate

## 🎉 [2.1.0] - 2026-09-01

### ✨ Features

- Add the cli form, with keyring and self-update as add-ons

### 📖 Documentation

- Record why optional pieces are add-ons, and release 2.1.0

### 🛠️ Bug Fixes

- Keep the generator building on the declared MSRV

## 🎉 [2.0.0] - 2026-09-01

### 💥 Breaking Changes

- **Replace the webpack wrapper with `lyrn new`**
`lyrn create`, `start`, `build` and `export` are removed along
with the webpack tooling behind them. Pin `lyrn@1.3.0` to keep the old
behaviour, or move to Vite directly - which is what the 1.x templates were
generating configuration for.


### ✨ Features

- Bring the repository up to the line's standard
- [**breaking**] Replace the webpack wrapper with `lyrn new`
- Add the brand assets

### 📖 Documentation

- Record the decisions behind 2.0 and regenerate the changelog
- Updated logo, updated information in README file

### 🛟 Miscellaneous Tasks

- Tea.yaml has been added

## 🎉 [1.3.0] - 2023-11-30

### ✨ Features

- Added method get_webpack_config to select WebpackFrameworkConfig
- Added WebpackFrameworkConfig
- Added framework definition to project config file
- Added component generation functions to Vue 3 template
- Updated Vue 3 template
- Added Value-based trait for merging non-null values
- Added Vue 3 template

### 🎛️ Refactor

- Improved ProjectConfig
- Improved React template
- Styles are moved to a separate module

### 🛠️ Bug Fixes

- Small fixes
- Added template for src/vue.d.ts

### 🧪 Testing

- Updated examples

## 🎉 [1.2.2] - 2023-11-21

### 🎛️ Refactor

- Improved performance of the ProjectAliases module
- ProjectAliases is moved to a separate module

### 🛠️ Bug Fixes

- Fixed function "format_str". Updated example config files.
- Fixed function "format_str". Fixed tests

### 🧪 Testing

- Fixed tests

## 🎉 [1.2.1] - 2023-11-09

### ✨ Features

- Added "show" flag to output the configuration to the console

### 🎛️ Refactor

- ProjectConfig is moved to a separate module

### 🛟 Miscellaneous Tasks

- Fix github action "documentation"

## 🎉 [1.2.0] - 2023-11-08

### ✨ Features

- Added merging of custom and default webpack configurations using the "webpack-merge" package
- Added generation of path to configuration files to the file "lyrn.json"
- Improved "export" сommand
- Added a new command called "export"

### 🎛️ Refactor

- Minor edits
- Constants are placed in a separate data array

### 🎨 Styling

- Changed emoji

### 📖 Documentation

- Added quick reference information about the "export" command

### 🛟 Miscellaneous Tasks

- Fix github action "documentation"
- Added basic command descriptions
- Fix github action "documentation"
- Fix github action "documentation"
- Fix github action "documentation"
- Fix github action "documentation"
- Fix .gitignore
- Fix github action "documentation"
- Fix github action "documentation"
- Fix github action "documentation"
- Fix github action "documentation"

### 🛠️ Bug Fixes

- Fixed function "aliases_json". Minor edits

### 🧪 Testing

- Added unit tests for webpack functions

## 🎉 [1.1.3] - 2023-10-18

### 📖 Documentation

- Added lyrn documentation

### 🛟 Miscellaneous Tasks

- Fix github action "documentation"
- Add github action "documentation"
- Removed unused functions in helpers

### 🧪 Testing

- Added unit tests for project functions
- Added unit tests for helper function "get_git_user"
- Added unit tests for helper functions

## 🎉 [1.1.2] - 2023-09-30

### ✨ Features

- Added a port argument to the "start" command
- Added lyrn.json file generation
- Added function for reading config file lyrn.json

### 🎛️ Refactor

- Project_config moved to templates
- AppConfig replaced by ProjectConfig

### 🛠️ Bug Fixes

- Fixed adding a port argument to the "start" command

## 🎉 [1.1.1] - 2023-09-23

### ✨ Features

- Added function for collecting paths from a tsconfig.json file

### 🎛️ Refactor

- Changed variable name "webpack" to "webpack_config"

### 📖 Documentation

- Added information to the README file

### 🛠️ Bug Fixes

- Fixed generation of paths in tsconfig

## 🎉 [1.1.0] - 2023-09-12

### ✨ Features

- Added command to build build on webpack
- Added an argument to run the "start" script in webpack

### 🛠️ Bug Fixes

- Fixed the message about the end of the build

## 🎉 [1.0.4] - 2023-09-06

### 📖 Documentation

- Clarified information in README.md

### 🛠️ Bug Fixes

- Added handling of file missing error when executing "start" command

## 🎉 [1.0.3] - 2023-09-05

### 📖 Documentation

- Added information to the README file

### 🛠️ Bug Fixes

- Fixed bug with displaying content not full screen

## 🎉 [1.0.2] - 2023-09-04

### 🛟 Miscellaneous Tasks

- Fix github action "release"

## 🎉 [1.0.1] - 2023-09-03

### 🛠️ Bug Fixes

- The version of lyrn in the package.json has been changed to 1.0.0

## 🎉 [1.0.0] - 2023-09-02

### 💥 Breaking Changes

- **📦✨ next lyrn version started**
📦✨ next lyrn version started


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

## 🎉 [0.0.9] - 2023-09-02

### 🎨 Styling

- 🖼️ changed logo

## 🎉 [0.0.2] - 2018-04-18


