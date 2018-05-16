const path = require('path');
const { remove } = require('fs-extra');
const clone = require('../util/clone');

const download = async (repo, targetPath) => {
  try {
    const results = await clone(repo, targetPath);
    remove(path.join(targetPath, '.git'));
    return results;
  } catch (error) {
    throw error;
  }
};

exports.api = download;

const cli = async (repo, targetPath) => {
  try {
    const results = await download(repo, targetPath);
    console.log(`Repository ${results} successfully downloaded`);
  } catch (error) {
    throw error;
  }
};

exports.cli = cli;
