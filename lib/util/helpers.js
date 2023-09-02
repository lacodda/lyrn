/**
 * get full repo url
 * @param origin
 * @returns {*}
 */
module.exports.getRepoUrl = (origin) => {
  const getGithubUrl = blueprint => `https://github.com/${blueprint}.git`;
  let result = '';

  if (/^(git@|https?:\/\/)/i.test(origin)) {
    result = origin;
  } else if (/\//i.test(origin)) {
    result = getGithubUrl(origin);
  } else {
    result = getGithubUrl(`lacodda/${origin}`);
  }

  return result;
};
