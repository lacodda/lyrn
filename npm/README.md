<p align="center">
  <img src="https://raw.githubusercontent.com/lacodda/lyrn/main/lyrn.webp" width="320" alt="lyrn">
</p>
<h1 align="center">Quick start of your web application</h1>
<br>

[![NPM Version][npm-image]][npm-url]

## Description

Lyrn is a command-line tool for quickly starting the development of your web application.

## Install

You can run the application using the `npx` utility:

```bash
npx lyrn@latest
```

Or install the package from npm globally

```bash
npm i -g lyrn
```

This will make the `lyrn` application globally available in your system

## Usage

To create an application, use the command:

```bash
lyrn create project your-app-name -f react
```

This command will create a simple application using the React framework

## Development

Navigate to your application folder

```bash
cd ./your-app-name
```

To start the development server, use the command:

```bash
npm start
```

## Build

Navigate to your application folder

```bash
cd ./your-app-name
```

To build your project, use the command:

```bash
npm run build
```

To run the built project, use the command:

```bash
npm run serve
```

## License

[MIT](LICENSE)

[npm-image]: https://img.shields.io/npm/v/lyrn.svg?style=flat-square
[npm-url]: https://npmjs.org/package/lyrn