'use strict';

const { promisify } = require('util');
const { exec } = require('child_process');

// const exec = util(exec);

module.exports = async command => {
  const { stdout, stderr } = await promisify(exec)(command);
  return { 'stdout': stdout, 'stderr': stderr };
};
