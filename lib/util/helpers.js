/**
 * get full repo url
 * @param origin
 * @returns {*}
 */
module.exports.getRepoUrl = (origin) => {
  const getGithubUrl = blueprint => `https://github.com/${blueprint}.git`;

  if (/^(git@|https?:\/\/)/i.test(origin)) {
    return origin;
  } else if (/\//i.test(origin)) {
    return getGithubUrl(origin);
  }

  return getGithubUrl(`lacodda/${origin}`);
};
