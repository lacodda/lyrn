'use strict';

const { promisify } = require('util');
const { exec } = require('child_process');

module.exports = async command => {
  const { stdout, stderr } = await promisify(exec)(command);
  return { 'stdout': stdout, 'stderr': stderr };
};

/**
 *
 * @param command
 * @returns {Promise<*>}
 */
module.exports.new = async command => {
  const { stdout, stderr } = await promisify(exec)(command);

  if (stderr) {
    throw new Error(stderr);
  }

  return stdout;
};
