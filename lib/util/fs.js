const { promisify } = require('util');
const { readdir } = require('fs');

/**
 * async/await fs readdir
 * @param dir
 * @returns {Promise<*>}
 */
module.exports.readdir = async (dir) => {
  try {
    return await promisify(readdir)(dir);
  } catch (error) {
    throw error;
  }
};
