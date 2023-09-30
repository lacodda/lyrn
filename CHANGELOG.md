# Changelog

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

- Added command to build on webpack
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

