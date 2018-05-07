/* eslint-disable no-unused-vars */

process.env.NODE_ENV = 'test';
const chai = require('chai');
const path = require('path');
const { remove } = require('fs-extra');
const { spawn } = require('child_process');

// globals
global.spawn = spawn;
global.expect = chai.expect;
global.chai = chai;
global.testTempFolder = 'tmp';
global.testPath = path.join(__dirname, testTempFolder);
global.createFilePath = (name) => path.join(__dirname, testTempFolder, name);
global.removeFolder = (name) => remove(name).catch(err => {
  console.log(err);
  throw err;
});
global.repoUrl = 'https://github.com/lacodda/test.git';
