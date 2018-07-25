/* eslint-disable no-unused-vars */

process.env.NODE_ENV = 'test';
const pkg = require('../package.json');
const { join, resolve } = require('path');
const chai = require('chai');

// settings
const bin = 'node ./bin/lyrn';
const tmpDir = 'tmp';
const repoUrl = 'https://github.com/lacodda/test.git';

// globals
global.pkg = pkg;
global.assert = chai.assert;
global.expect = chai.expect;
global.chai = chai;
global.bin = bin;
global.repoUrl = repoUrl;
global.tmpPath = resolve(join(__dirname, tmpDir));

global.getTmpPath = (name) => resolve(join(tmpPath, name));
