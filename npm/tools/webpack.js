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

  const devServerOptions = config.devServer;
  const compiler = webpack(config);
  const server = new WebpackDevServer(devServerOptions, compiler);

  compiler.hooks.done.tap('serve', (stats) => {
    if (stats.hasErrors()) {
      return;
    }
    console.log('done');
  })
  compiler.hooks.compile.tap('serve', () => {
    console.log('compile');
  })
  server.start(app_config.port, `${app_config.protocol}://${app_config.host}`, (err) => {
    if (err) {
      process.exit(0);
    }
  })
});
