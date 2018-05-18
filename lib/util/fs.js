const { promisify } = require('util');
const { readdir, access } = require('fs');
const { remove } = require('fs-extra');

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

/**
 * async/await fs access
 * @param path
 * @returns {Promise<*>}
 */
module.exports.access = async (path) => {
  try {
    return await promisify(access)(path);
  } catch (error) {
    throw error;
  }
};

/**
 * async/await fs remove recursively
 * @param path
 * @returns {Promise<*>}
 */
module.exports.remove = async (path) => {
  try {
    return await promisify(remove)(path);
  } catch (error) {
    throw error;
  }
};
