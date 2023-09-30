use super::{AppConfig, DevConfig, ProdConfig, ProjectConfig, ProjectProps, Template};
use crate::libs::types::User;
use chrono::Datelike;
use serde_json::{json, Value};

pub fn get(project: &ProjectProps) -> Template {
    Template {
        scripts: scripts(),
        dependencies: dependencies(),
        dev_dependencies: dev_dependencies(),
        project_config: project_config(),
        tsconfig: tsconfig(),
        eslintrc: eslintrc(),
        readme: readme(&project),
        mit_license: mit_license(&project.user),
        gitignore: gitignore(),
        postcss_config: postcss_config(),
        index_d: index_d(),
        index: index(&project.name),
        ..Template::default()
    }
}

fn scripts() -> Value {
    json!({
        "start": "lyrn start",
        "build": "lyrn build",
        "serve": "serve dist"
    })
}

fn dependencies() -> Value {
    json!({"lyrn": "^1.0.0"})
}

fn dev_dependencies() -> Value {
    json!({
        "@types/node": "^20.4.2",
        "@types/webpack-env": "^1.18.1",
        "@typescript-eslint/eslint-plugin": "^6.0.0",
        "@typescript-eslint/parser": "^6.0.0",
        "babel-loader": "^9.1.3",
        "copy-webpack-plugin": "^11.0.0",
        "css-loader": "^6.8.1",
        "dotenv": "^16.3.1",
        "eslint": "^8.45.0",
        "eslint-import-resolver-alias": "^1.1.2",
        "eslint-import-resolver-webpack": "^0.13.2",
        "eslint-plugin-import": "^2.27.5",
        "file-loader": "^6.2.0",
        "fork-ts-checker-webpack-plugin": "^8.0.0",
        "html-webpack-plugin": "^5.5.3",
        "mini-css-extract-plugin": "^2.7.6",
        "postcss-loader": "^7.3.3",
        "postcss-preset-env": "^9.0.0",
        "sass": "^1.63.6",
        "sass-loader": "^13.3.2",
        "serve": "^14.2.0",
        "style-loader": "^3.3.3",
        "ts-loader": "9.4.4",
        "typescript": "^5.1.6",
        "webpack": "^5.88.1",
        "webpack-cli": "^5.1.4",
        "webpack-dev-server": "^4.15.1",
        "webpack-shell-plugin-next": "^2.3.1",
        "zx": "^7.2.3"
    })
}

pub fn project_config() -> ProjectConfig {
    ProjectConfig {
        app: AppConfig {
            name: "app".into(),
            title: "New application".into(),
            ..Default::default()
        },
        dev: DevConfig {
            public_path: "/".into(),
            protocol: "http".into(),
            host: "localhost".into(),
            port: 8080,
            ..Default::default()
        },
        prod: ProdConfig {
            public_path: "/".into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn tsconfig() -> Value {
    json!({
        "compileOnSave": false,
        "compilerOptions": {
          "rootDir": ".",
          "sourceMap": true,
          "declaration": false,
          "moduleResolution": "node",
          "emitDecoratorMetadata": true,
          "experimentalDecorators": true,
          "importHelpers": true,
          "target": "ESNext",
          "module": "ESNext",
          "lib": ["ESNext", "dom"],
          "skipLibCheck": true,
          "skipDefaultLibCheck": true,
          "esModuleInterop": true,
          "noImplicitAny": true,
          "resolveJsonModule": true,
          "baseUrl": ".",
          "outDir": "dist",
          "paths": {
            "@/*": ["src/*"]
          }
        },
        "exclude": ["node_modules", "dist"]
    })
}

fn eslintrc() -> Value {
    json!({
        "extends": [
          "eslint:recommended",
          "standard-with-typescript"
        ],
        "rules": {
          "arrow-body-style": 1,
          "camelcase": 1,
          "comma-dangle": ["error", "always-multiline"],
          "import/no-extraneous-dependencies": ["off", { "devDependencies": false }],
          "import/prefer-default-export": 0,
          "indent": ["error", 2, { "SwitchCase": 1 }],
          "linebreak-style": ["error", "unix"],
          "object-curly-spacing": ["warn", "always"],
          "prefer-template": "off",
          "semi": ["error", "always"],
          "quotes": ["error", "single"],
          "no-var": 1,
          "no-unused-vars": 1,
          "no-nested-ternary": 1,
          "no-console": 1,
          "no-template-curly-in-string": 1,
          "no-self-compare": 1,
          "no-async-promise-executor": "warn",
          "no-debugger": "warn",
          "no-multi-spaces": ["error"],
          "no-prototype-builtins": "off",
          "no-undef": "warn",
          "@typescript-eslint/comma-dangle": ["error", "always-multiline"],
          "@typescript-eslint/indent": ["error", 2, { "SwitchCase": 1 } ],
          "@typescript-eslint/no-unused-vars": ["warn", { "vars": "all", "args": "none", "ignoreRestSiblings": false }],
          "@typescript-eslint/no-inferrable-types": "off",
          "@typescript-eslint/semi": ["error", "always"],
          "@typescript-eslint/space-before-function-paren": "off",
          "@typescript-eslint/strict-boolean-expressions": "off",
          "@typescript-eslint/member-delimiter-style": ["error",
            {
              "multiline": { "delimiter": "semi" },
              "singleline": { "delimiter": "comma", "requireLast": false }
            }
          ],
          "@typescript-eslint/no-empty-interface": ["error", { "allowSingleExtends": true }],
          "@typescript-eslint/no-extraneous-class": "off",
          "@typescript-eslint/prefer-nullish-coalescing": "off"
        },
        "ignorePatterns": ["dist", "node_modules"],
        "env": {
          "browser": true,
          "es6": true,
          "node": true
        },
        "parserOptions": {
          "ecmaVersion": "latest",
          "sourceType": "module",
          "project": "./tsconfig.json"
        },
        "settings": {
          "import/resolver": {
            "node": {
              "extensions": [".js", ".jsx", ".ts", ".tsx"]
            }
          }
        }
    })
}

fn readme(project: &ProjectProps) -> String {
    format!(
        r###"# {}

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

## Project setup
```
npm install
```

## Usage

### Development server
```
npm start
```

### Production build
```
npm run build
```

You can view the deploy by creating a server in `dist`
```
npm run serve
```

### Lints and fixes files
```
npm run lint
```

## Author

- [{}](https://my-site.com)

## License

This project is open source and available under the [MIT License](LICENSE).
"###,
        project.name, project.user.name
    )
}

fn mit_license(user: &User) -> String {
    let now = chrono::Utc::now();
    format!(
        r###"MIT License

Copyright (c) {} {} <{}>

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"###,
        now.year(),
        user.name,
        user.email,
    )
}

fn gitignore() -> String {
    r###"# See http://help.github.com/ignore-files/ for more about ignoring files.

# compiled output
dist
tmp
/out-tsc

# dependencies
node_modules

# IDEs and editors
/.idea
.project
.classpath
.c9/
*.launch
.settings/
*.sublime-workspace

# IDE - VSCode
.vscode/*
!.vscode/settings.json
!.vscode/tasks.json
!.vscode/launch.json
!.vscode/extensions.json

# misc
/.sass-cache
/connect.lock
/coverage
/libpeerconnection.log
npm-debug.log
yarn-error.log
testem.log
/typings

# System Files
.DS_Store
Thumbs.db
"###
    .into()
}

fn postcss_config() -> String {
    r###"module.exports = {
  plugins: {
    'postcss-preset-env': {
      browsers: 'last 2 versions',
    },
  },
}; 
"###
    .into()
}

fn index_d() -> String {
    r###"declare module '*.css';
declare module '*.png';
declare module '*.jpg';
declare module '*.jpeg';
declare module '*.svg'; 
"###
    .into()
}

fn index(name: &String) -> String {
    format!(
        r###"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta http-equiv="X-UA-Compatible" content="IE=edge">
  <meta name="viewport" content="width=device-width,initial-scale=1.0">
  <title>{}</title>
</head>
<body>
  <noscript>
    <strong>We're sorry but {} doesn't work properly without JavaScript enabled. Please enable it to continue.</strong>
  </noscript>
  <div id="app"></div>
  <!-- built files will be auto injected -->
</body>
</html>
"###,
        name, name
    )
}
