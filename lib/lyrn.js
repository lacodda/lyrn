'use strict';

const fs = require('fs');
const pkg = require('../package.json');

const lyrn = { loaded: false };
lyrn.version = pkg.version;

const api = {}, cli = {};

Object.defineProperty(lyrn, 'commands', {
  get: () => {
    if (lyrn.loaded === false) {
      throw new Error('run lyrn.load before');
    }
    return api;
  }
});

Object.defineProperty(lyrn, 'cli', {
  get: () => {
    if (lyrn.loaded === false) {
      throw new Error('run lyrn.load before');
    }
    return cli;
  }
});

lyrn.load = function load (opts) {
  return new Promise((resolve, reject) => {
    fs.readdir(__dirname, (err, files) => {
      files.forEach((file) => {
        if (!/\.js$/.test(file) || file === 'lyrn.js') {
          return;
        }

        const cmd = file.match(/(.*)\.js$/)[1];
        const mod = require('./' + file);
        if (mod.cli) {
          cli[cmd] = mod.cli;
        }

        if (mod.api) {
          api[cmd] = mod.api;
        }

      });
      lyrn.loaded = true;
      resolve(lyrn);
    });
  });
};

module.exports = lyrn;
