'use strict'

const webpack = require('webpack');
const WebpackDevServer = require('webpack-dev-server');

process.stdin.on('data', function (inputData) {
  const { config, app_config, plugins, rules } = JSON.parse(inputData);

  for (const rule of rules) {
    config.module.rules.push(eval(rule));
  }
  for (const plugin of plugins) {
    config.plugins.push(eval(plugin));
  }

  switch (true) {
    case process.argv.includes('start'):
      start({ config, app_config });
      break;
    case process.argv.includes('build'):
      build({ config, app_config });
      break;
    default:
      process.exit(0);
  }
});

function start({ config, app_config }) {
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
  server.start(app_config.port, `${app_config.protocol}://${app_config.host}`, (err) => {
    if (err) {
      process.exit(0);
    }
  });
}

function build({ config, app_config }) {
  process.exit(0);
}