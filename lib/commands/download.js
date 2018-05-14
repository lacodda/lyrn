const { remove } = require('fs-extra');
const clone = require('../util/clone');

const download = async (repo, targetPath) => {
  try {
    const results = await clone(repo, targetPath);
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
    return error;
  }
};

exports.cli = cli;
