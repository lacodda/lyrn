const { join, resolve } = require('path');
const { remove } = require('fs-extra');
const clone = require('../util/clone');

exports.command = 'download <repo> <targetPath>';
exports.desc = 'Download the git repository';
exports.builder = {
  repo: {
    alias: 'r'
  },
  targetPath: {
    alias: 't'
  }
};

/**
 * cli download command
 * @param {*} argv
 */
exports.handler = async (argv) => {
  try {
    const results = await download(argv.repo, argv.targetPath);
    console.log(`Repository ${results} successfully downloaded`);
  } catch (error) {
    throw error;
  }
};

/**
 * api download function
 * @param repo
 * @param targetPath
 * @returns {Promise<*>}
 */
const download = async (repo, targetPath) => {
  try {
    const results = await clone(repo, targetPath);
    await remove(resolve(join(targetPath, '.git')));
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
