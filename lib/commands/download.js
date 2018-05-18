const path = require('path');
const clone = require('../util/clone');
const { remove } = require('../util/fs');

/**
 * api download function
 * @param repo
 * @param targetPath
 * @returns {Promise<*>}
 */
const download = async (repo, targetPath) => {
  try {
    const results = await clone(repo, targetPath);
    await remove(path.join(targetPath, '.git'));
    return results;
  } catch (error) {
    throw error;
  }
};

exports.api = download;

/**
 * cli download command
 * @param repo
 * @param targetPath
 * @returns {Promise<void>}
 */
const cli = async (repo, targetPath) => {
  try {
    const results = await download(repo, targetPath);
    console.log(`Repository ${results} successfully downloaded`);
  } catch (error) {
    throw error;
  }
};

exports.cli = cli;
