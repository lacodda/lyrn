const { promisify } = require('util');
const { exec } = require('child_process');

/**
 *
 * @param command
 * @returns {Promise<*>}
 */
module.exports = async (command) => {
  const { error, stdout, stderr } = await promisify(exec)(command);

  if (error) {
    throw new Error(stderr);
  }

  return stdout;
};
