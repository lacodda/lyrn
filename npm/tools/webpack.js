'use strict'

const webpack = require('webpack');
const WebpackDevServer = require('webpack-dev-server');
const { resolve } = require('path');
const { mergeWithCustomize, customizeArray, } = require('webpack-merge');

process.stdin.on('data', function (inputData) {
  const { config, project_config, constants, plugins, rules } = JSON.parse(inputData);

  for (const constant of constants) {
    eval(`global.${constant}`);
  }
  for (const rule of rules) {
    config.module.rules.push(eval(rule));
  }
  for (const plugin of plugins) {
    config.plugins.push(eval(plugin));
  }

  switch (true) {
    case process.argv.includes('start'):
      start({ config, project_config });
      break;
    case process.argv.includes('build'):
      build({ config, project_config });
      break;
    default:
      process.exit(0);
  }
});

function start({ config, project_config }) {
  config = getConfig(config, project_config.dev.config);
  const devServerOptions = config.devServer;
  const compiler = webpack(config);
  const server = new WebpackDevServer(devServerOptions, compiler);

  compiler.hooks.done.tap('serve', (stats) => {
    if (stats.hasErrors()) {
      return;
    }
    console.log('done');
  });
  compiler.hooks.compile.tap('serve', () => {
    console.log('compile');
  });
  server.start(project_config.dev.port, `${project_config.dev.protocol}://${project_config.dev.host}`, (err) => {
    if (err) {
      process.exit(0);
    }
  });
}

function build({ config, project_config }) {
  config = getConfig(config, project_config.prod.config);
  webpack(config, (err, stats) => {
    if (err || stats.hasErrors()) {
      process.exit(0);
    }

    console.log('done',
      JSON.stringify(stats.toJson({
        colors: true,
        modules: false,
        children: false,
        chunks: false,
        chunkModules: false,
      })));
  });
}

function getConfig(config, custom_config_path) {
  if (!custom_config_path) {
    return config;
  }
  const custom_config = require(resolve(custom_config_path));
  return mergeWithCustomize({
    customizeArray: customizeArray({
      entry: 'replace',
      plugins: 'replace',
      'module.rules': 'replace',
    })
  })(config, custom_config);
}