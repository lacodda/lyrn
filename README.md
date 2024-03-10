<p align="center">
  <img src="https://raw.githubusercontent.com/lacodda/lyrn/main/lyrn.webp" width="320" alt="lyrn">
</p>
<h1 align="center">🚀 Quick Start for Web Application Development with lyrn</h1>
<br>

[![NPM Version][npm-image]][npm-url] ![Build Status][build-image] ![Contributors][contributors-image] ![License][license-url]

## 📌 Overview

lyrn is an innovative command-line tool that accelerates the web application development process. It simplifies the initial setup and configuration, allowing developers to focus on creating robust and high-quality web applications. lyrn supports a variety of frameworks and tools, making it versatile for different project requirements.

## 🌟 Features

- **Framework Agnostic:** Start projects with any major web framework, including React, Vue, Svelte, and more.
- **Instant Setup:** Automates the configuration of development environments, including necessary dependencies and build tools.
- **Modern Tooling:** Integrates with modern development tools such as Webpack, Babel, ESLint, and TypeScript, ensuring best practices and optimal development workflows.
- **Custom Templates:** Support for custom project templates, allowing teams to standardize development practices.
- **Live Reloading:** Facilitates development with live reloading, ensuring immediate feedback for any changes.

## 🚀 Getting Started

### Prerequisites

Before installing lyrn, make sure you have Node.js and npm installed on your system. These are essential for running JavaScript projects and managing their dependencies.

### Installation

To install lyrn globally on your system, use the following npm command:

```bash
npm install -g lyrn
```

This allows you to use lyrn from any directory on your system.

Alternatively, for a one-off project setup without global installation, use:

```bash
npx lyrn@latest
```

### Creating Your First Project

To create a new project, lyrn streamlines the process with a simple command:

```bash
lyrn create project <your-app-name> -f <framework>
```

Replace `<your-app-name>` with the name of your project and `<framework>` with the web framework you want to use (e.g., `react`, `vue`).

## 🛠 Usage

After creating a project, navigate into your project directory to start development:

```bash
cd <your-app-name>
```

To launch the development server with live reloading:

```bash
npm start
```

When you're ready to build your application for production:

```bash
npm run build
```

To serve your production build locally:

```bash
npm run serve
```

## 💡 Contributing

We welcome contributions from the community! Whether it's adding new features, fixing bugs, or improving documentation, your help is appreciated. Please read our [Contributing Guide](CONTRIBUTING.md) for more details on how to start contributing.

## 🆘 Support

If you encounter any issues or have questions, please file an issue on our GitHub repository.

## 📜 License

lyrn is open-source software licensed under the MIT license. See the [LICENSE](LICENSE) file for more details.

[build-image]: https://img.shields.io/travis/com/lacodda/lyrn/main.svg?style=flat-square
[build-url]: https://travis-ci.com/lacodda/lyrn
[contributors-image]: https://img.shields.io/github/contributors/lacodda/lyrn.svg?style=flat-square
[contributors-url]: https://github.com/lacodda/lyrn/graphs/contributors
[license-image]: https://img.shields.io/github/license/lacodda/lyrn.svg?style=flat-square
[license-url]: https://github.com/lacodda/lyrn/blob/main/LICENSE
[npm-image]: https://img.shields.io/npm/v/lyrn.svg?style=flat-square
[npm-url]: https://npmjs.org/package/lyrn
