'use strict';

const util = require('util').promisify;
const exec = require('child_process').exec;

// const exec = util(exec);

module.exports = async command => {
  const { stdout } = await util(exec)(command);
  return stdout;
};
