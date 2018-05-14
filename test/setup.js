/* eslint-disable no-unused-vars */

process.env.NODE_ENV = 'test';
const pkg = require('../package.json');
const path = require('path');
const chai = require('chai');
const { remove } = require('fs-extra');
const { spawn } = require('child_process');

// globals
global.pkg = pkg;
global.spawn = spawn;
global.assert = chai.assert;
global.expect = chai.expect;
global.chai = chai;
global.lyrn = './bin/lyrn';
global.testTempFolder = 'tmp';
global.testPath = path.join(__dirname, testTempFolder);
global.createFilePath = (name) => path.join(__dirname, testTempFolder, name);
global.removeFolder = (name) => remove(name).catch(err => {
  console.log(err);
  throw err;
});
global.repoUrl = 'https://github.com/lacodda/test.git';
