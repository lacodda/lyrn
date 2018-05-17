const exec = require('./exec');

/**
 * git clone function
 * @param repo
 * @param targetPath
 * @param branch
 * @returns {Promise<*>}
 */
module.exports = async (repo, targetPath, branch = null) => {
  let cloneCommand = 'git clone --depth=1';

  if (branch) {
    cloneCommand = `${cloneCommand} --branch ${branch}`;
  }

  cloneCommand = `${cloneCommand} ${repo} "${targetPath}"`;

  try {
    await exec(cloneCommand);

    return repo;
  } catch (error) {
    throw new Error(`git clone failed with status ${error}`);
    // throw error;
  }
};
